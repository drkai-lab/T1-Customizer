#!/usr/bin/env python3
"""Post a CI failure to ntfy.

The topic is read from the NTFY_TOPIC environment variable. This file does
not contain a topic. The value is not printed.

A failed job whose steps never received a conclusion is a billing or
runner problem (zero steps), not a test or build failure.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.error
import urllib.parse
import urllib.request

DONE = {"success", "failure", "cancelled", "skipped"}


def executed_steps(job):
    return [step for step in (job.get("steps") or []) if step.get("conclusion") in DONE]


def failed_jobs(jobs):
    selected = []
    for job in jobs:
        steps = job.get("steps") or []
        has_failed_step = any(step.get("conclusion") == "failure" for step in steps)
        if job.get("conclusion") == "failure" or has_failed_step:
            selected.append(job)
    return selected


def classify_jobs(jobs):
    failed = failed_jobs(jobs)
    if not failed:
        return "none"
    silent = [job for job in failed if not executed_steps(job)]
    noisy = [job for job in failed if executed_steps(job)]
    if noisy and silent:
        return "mixed"
    if silent:
        return "environment"
    return "steps"


def render_message(kind, jobs, context):
    failed = failed_jobs(jobs)
    url = context.get("run_url") or ""
    repo = context.get("repo") or ""
    branch = context.get("branch") or ""
    sha = (context.get("sha") or "")[:7]
    if kind == "environment":
        names = ", ".join(job.get("name") or "?" for job in failed) or "(ジョブなし)"
        return (
            "T1-VREmoKey CI がステップ 0 件で失敗しました。\n"
            "課金または実行環境の問題です（GitHub ホスト型ランナーの分数切れ、"
            "self-hosted runner の停止、ジョブ開始前の失敗）。"
            "テストやビルドの失敗ではありません。\n"
            f"対象: {names}\n"
            f"{repo} {branch} {sha}\n"
            f"{url}"
        )
    lines = []
    for job in failed:
        bad = [
            step.get("name") or "?"
            for step in (job.get("steps") or [])
            if step.get("conclusion") == "failure"
        ]
        if not bad and not executed_steps(job):
            lines.append(f"{job.get('name') or '?'}: ステップ 0 件（課金または実行環境）")
        else:
            lines.append(f"{job.get('name') or '?'}: {', '.join(bad) if bad else '失敗'}")
    return "T1-VREmoKey CI が失敗しました。\n" + "\n".join(lines) + f"\n{repo} {branch} {sha}\n{url}"


def _next_link(header):
    if not header:
        return None
    for part in header.split(","):
        section = part.strip()
        if 'rel="next"' not in section:
            continue
        start = section.find("<")
        end = section.find(">")
        if start != -1 and end != -1:
            return section[start + 1 : end]
    return None


def fetch_jobs(repo, run_id, token):
    jobs = []
    url = f"https://api.github.com/repos/{repo}/actions/runs/{run_id}/jobs?per_page=100"
    while url:
        request = urllib.request.Request(
            url,
            headers={
                "Accept": "application/vnd.github+json",
                "Authorization": f"Bearer {token}",
                "User-Agent": "t1-vremokey-ci-notify",
                "X-GitHub-Api-Version": "2022-11-28",
            },
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            payload = json.loads(response.read().decode("utf-8"))
            link = response.headers.get("Link")
        jobs.extend(payload.get("jobs") or [])
        url = _next_link(link)
    return jobs


def post_ntfy(topic, message):
    url = "https://ntfy.sh/" + urllib.parse.quote(topic, safe="")
    request = urllib.request.Request(
        url,
        data=message.encode("utf-8"),
        headers={
            "Title": "T1-VREmoKey CI",
            "Content-Type": "text/plain; charset=utf-8",
        },
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=20) as response:
        response.read()


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--only-environment", action="store_true")
    parser.add_argument("--run-id", default=os.environ.get("GITHUB_RUN_ID", ""))
    args = parser.parse_args(argv)
    topic = os.environ.get("NTFY_TOPIC", "").strip()
    if not topic:
        print("NTFY_TOPIC is unset; skipping notification", file=sys.stderr)
        return 0
    repo = os.environ.get("GITHUB_REPOSITORY", "").strip()
    token = (os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN") or "").strip()
    run_id = str(args.run_id).strip()
    if not repo or not token or not run_id:
        print("missing repo, token, or run id; skipping notification", file=sys.stderr)
        return 0
    try:
        jobs = fetch_jobs(repo, run_id, token)
    except (urllib.error.URLError, TimeoutError, json.JSONDecodeError, OSError) as exc:
        print(f"could not read workflow jobs: {exc}", file=sys.stderr)
        return 0
    kind = classify_jobs(jobs)
    if kind == "none":
        print("no failed jobs to report")
        return 0
    if args.only_environment and kind != "environment":
        print("steps ran; the job notify step reports those")
        return 0
    rendered = "environment" if kind == "environment" else "steps"
    server = os.environ.get("GITHUB_SERVER_URL", "https://github.com").rstrip("/")
    run_url = os.environ.get("CI_RUN_URL") or f"{server}/{repo}/actions/runs/{run_id}"
    message = render_message(
        rendered,
        jobs,
        {
            "run_url": run_url,
            "repo": repo,
            "branch": os.environ.get("GITHUB_REF_NAME", ""),
            "sha": os.environ.get("GITHUB_SHA", ""),
        },
    )
    try:
        post_ntfy(topic, message)
    except (urllib.error.URLError, TimeoutError, OSError) as exc:
        print(f"ntfy post failed: {exc}", file=sys.stderr)
        return 0
    print("notified ntfy")
    return 0


if __name__ == "__main__":
    sys.exit(main())
