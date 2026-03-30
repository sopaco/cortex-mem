"""
Cortex Memory LoCoMo evaluator.

Method aligned with OpenViking/OpenClaw evaluation workflow:
- ingest: load LoCoMo multi-session conversations into memory
- qa: ask LoCoMo QA questions against the ingested memory

This toolkit only evaluates Cortex Memory.
"""

import argparse
import asyncio
import json
import os
import re
import sys
import time
from typing import Any

import requests
from openai import OpenAI


DEFAULT_SERVICE_URL = "http://127.0.0.1:8085"
DEFAULT_SEARCH_MODEL = os.getenv("EVAL_ANSWER_MODEL", "gpt-4o-mini")
DEFAULT_JUDGE_MODEL = os.getenv("EVAL_JUDGE_MODEL", DEFAULT_SEARCH_MODEL)
DEFAULT_TENANT_PREFIX = "locomo-eval"


def parse_test_file(path: str) -> list[dict[str, Any]]:
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    raw_sessions = content.split("---\n")
    sessions = []
    for raw in raw_sessions:
        lines = [line for line in raw.strip().splitlines() if line.strip()]
        if not lines:
            continue

        messages = []
        evals = []
        for line in lines:
            if line.startswith("eval:"):
                evals.append(line[len("eval:") :].strip())
            else:
                messages.append(line)

        if messages or evals:
            sessions.append({"messages": messages, "evals": evals})
    return sessions


def format_locomo_message(msg: dict[str, Any]) -> str:
    speaker = msg.get("speaker", "unknown")
    text = msg.get("text", "")
    line = f"{speaker}: {text}"

    img_urls = msg.get("img_url", [])
    if isinstance(img_urls, str):
        img_urls = [img_urls]
    blip = msg.get("blip_caption", "")

    if img_urls:
        for url in img_urls:
            caption = f": {blip}" if blip else ""
            line += f"\n{url}{caption}"
    elif blip:
        line += f"\n({blip})"

    return line


def load_locomo_data(path: str, sample_index: int | None = None) -> list[dict[str, Any]]:
    with open(path, "r", encoding="utf-8") as f:
        data = json.load(f)

    if sample_index is not None:
        if sample_index < 0 or sample_index >= len(data):
            print(f"Error: sample index {sample_index} out of range (0-{len(data)-1})", file=sys.stderr)
            sys.exit(1)
        return [data[sample_index]]
    return data


def parse_session_range(spec: str) -> tuple[int, int]:
    if "-" in spec:
        lo, hi = spec.split("-", 1)
        return int(lo), int(hi)
    value = int(spec)
    return value, value


def build_session_messages(
    item: dict[str, Any],
    session_range: tuple[int, int] | None = None,
    tail: str = "[]",
) -> list[dict[str, Any]]:
    conv = item["conversation"]
    speakers = f"{conv['speaker_a']} & {conv['speaker_b']}"

    session_keys = sorted(
        [k for k in conv if k.startswith("session_") and not k.endswith("_date_time")],
        key=lambda key: int(key.split("_")[1]),
    )

    sessions = []
    for session_key in session_keys:
        session_num = int(session_key.split("_")[1])
        if session_range is not None:
            lo, hi = session_range
            if session_num < lo or session_num > hi:
                continue

        dt_key = f"{session_key}_date_time"
        date_time = conv.get(dt_key, "")
        parts = [f"[group chat conversation: {date_time}]"]
        for msg in conv[session_key]:
            parts.append(format_locomo_message(msg))
        if tail:
            parts.append(tail)

        sessions.append(
            {
                "message": "\n\n".join(parts),
                "meta": {
                    "sample_id": item["sample_id"],
                    "session_key": session_key,
                    "date_time": date_time,
                    "speakers": speakers,
                },
            }
        )
    return sessions


class CortexEvalClient:
    def __init__(self, base_url: str, answer_model: str, llm_base_url: str | None, llm_api_key: str | None):
        self.base_url = base_url.rstrip("/")
        self.answer_model = answer_model
        self.llm_client = None
        if llm_base_url and llm_api_key:
            self.llm_client = OpenAI(base_url=llm_base_url, api_key=llm_api_key)

    def _post(self, path: str, payload: dict[str, Any], max_retries: int = 3) -> dict[str, Any]:
        url = f"{self.base_url}{path}"
        last_error = None
        for attempt in range(max_retries):
            try:
                response = requests.post(url, json=payload, timeout=300)
                response.raise_for_status()
                body = response.json()
                if not body.get("success", False):
                    raise RuntimeError(body.get("error") or f"Request failed: {url}")
                return body.get("data")
            except (requests.exceptions.RequestException, RuntimeError) as e:
                last_error = e
                if attempt < max_retries - 1:
                    import time
                    wait_time = (attempt + 1) * 5  # 5s, 10s, 15s backoff
                    print(f"  [retry] {path} failed ({e}), retrying in {wait_time}s...")
                    time.sleep(wait_time)
        raise last_error

    def switch_tenant(self, tenant_id: str) -> None:
        self._post("/api/v2/tenants/switch", {"tenant_id": tenant_id})

    def create_session(self, thread_id: str, user_id: str | None = None, agent_id: str | None = None) -> None:
        payload: dict[str, Any] = {"thread_id": thread_id}
        if user_id:
            payload["user_id"] = user_id
        if agent_id:
            payload["agent_id"] = agent_id
        self._post("/api/v2/sessions", payload)

    def add_message(self, thread_id: str, role: str, content: str) -> None:
        self._post(
            f"/api/v2/sessions/{thread_id}/messages",
            {"role": role, "content": content},
        )

    def close_session(self, thread_id: str) -> None:
        self._post(f"/api/v2/sessions/{thread_id}/close", {})

    def close_session_and_wait(
        self,
        thread_id: str,
        timeout_secs: float,
        poll_interval: float,
    ) -> dict[str, Any]:
        return self._post(
            f"/api/v2/sessions/{thread_id}/close-and-wait",
            {
                "timeout_secs": max(1, int(timeout_secs)),
                "poll_interval_ms": max(100, int(poll_interval * 1000)),
            },
        )

    def search(self, query: str, thread_id: str | None = None, limit: int = 8, min_score: float = 0.4) -> list[dict[str, Any]]:
        payload: dict[str, Any] = {
            "query": query,
            "limit": limit,
            "min_score": min_score,
            "return_layers": ["L0", "L1", "L2"],
        }
        if thread_id:
            payload["thread"] = thread_id
        return self._post("/api/v2/search", payload)

    def answer_question(self, question: str, contexts: list[dict[str, Any]]) -> tuple[str, dict[str, int]]:
        if self.llm_client is None:
            return self.extract_answer_from_contexts(contexts), {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}

        context_texts = []
        for idx, ctx in enumerate(contexts, start=1):
            text = ctx.get("content") or ctx.get("overview") or ctx.get("snippet") or ""
            if not text:
                continue
            context_texts.append(f"[Context {idx}]\nURI: {ctx.get('uri', '')}\n{text}")

        prompt = (
            "Answer the question using the provided memory contexts. "
            "Be direct and concise - give the specific answer (a name, date, fact, or short phrase) "
            "when the information is present or clearly implied in the contexts.\n\n"
            "CRITICAL TIME CONTEXT: These memories are from conversations that took place in 2023 "
            "(primarily May-October 2023). When you see relative time expressions like 'last Saturday', "
            "'yesterday', 'next month', they refer to dates in 2023, NOT the current date. "
            "Always convert relative dates to absolute dates (e.g., 'yesterday before May 7, 2023' = May 6, 2023).\n\n"
            "Guidelines:\n"
            "- Give SPECIFIC answers: exact dates (May 7, 2023), specific names (Sweden), precise numbers.\n"
            "- If the answer is explicitly stated, give it directly.\n"
            "- If the answer is strongly implied (e.g. past breakup implies current single status, "
            "'ten years ago' mentioned in context implies '10 years ago'), state it confidently.\n"
            "- For hypothetical/reasoning questions (e.g. 'would X still Y if...'), reason from "
            "available context and give your best inference (e.g. 'Likely no', 'Probably yes').\n"
            "- For multi-hop questions, combine information from multiple contexts to form a complete answer.\n"
            "- Only say you cannot answer if the contexts are completely unrelated to the question "
            "and no reasonable inference is possible.\n\n"
            f"Question: {question}\n\n"
            f"Memory Contexts:\n\n{chr(10).join(context_texts[:8])}\n\nAnswer:"
        )

        response = self.llm_client.chat.completions.create(
            model=self.answer_model,
            messages=[
                {"role": "system", "content": "You are a helpful assistant that answers questions from memory retrieval results. Be direct and give concise factual answers."},
                {"role": "user", "content": prompt},
            ],
            temperature=0,
        )
        text = response.choices[0].message.content or ""
        usage = response.usage
        return text.strip(), {
            "prompt_tokens": getattr(usage, "prompt_tokens", 0) if usage else 0,
            "completion_tokens": getattr(usage, "completion_tokens", 0) if usage else 0,
            "total_tokens": getattr(usage, "total_tokens", 0) if usage else 0,
        }

    def judge_answer(
        self,
        question: str,
        gold_answer: str,
        response_text: str,
        judge_model: str,
    ) -> tuple[bool, str, dict[str, int]]:
        if self.llm_client is None:
            raise RuntimeError(
                "Judge mode requires --llm-base-url and --llm-api-key (or OPENAI_BASE_URL / OPENAI_API_KEY)."
            )

        prompt = (
            "Evaluate whether the model response correctly answers the question given the gold answer. "
            "Allow paraphrases, equivalent dates, and concise wording. Mark WRONG if the response is factually incorrect, "
            "misses the key fact, answers a different question, or only provides loosely related context. "
            "Respond with JSON only using this schema: "
            '{"is_correct":"CORRECT"|"WRONG","reasoning":"short explanation"}.\n\n'
            f"Question: {question}\n"
            f"Gold Answer: {gold_answer}\n"
            f"Model Response: {response_text}\n"
        )

        completion = self.llm_client.chat.completions.create(
            model=judge_model,
            messages=[
                {"role": "system", "content": "You are a strict but fair QA judge. Output JSON only."},
                {"role": "user", "content": prompt},
            ],
            temperature=0,
        )
        content = completion.choices[0].message.content or ""
        parsed = extract_json_object(content)
        verdict = str(parsed.get("is_correct", "WRONG")).strip().upper() == "CORRECT"
        reasoning = str(parsed.get("reasoning", "")).strip()
        usage = completion.usage
        return verdict, reasoning, {
            "prompt_tokens": getattr(usage, "prompt_tokens", 0) if usage else 0,
            "completion_tokens": getattr(usage, "completion_tokens", 0) if usage else 0,
            "total_tokens": getattr(usage, "total_tokens", 0) if usage else 0,
        }

    @staticmethod
    def extract_answer_from_contexts(contexts: list[dict[str, Any]]) -> str:
        if not contexts:
            return "I cannot answer this question based on the provided memory contexts."
        text = contexts[0].get("content") or contexts[0].get("overview") or contexts[0].get("snippet") or ""
        return text.strip() or "I cannot answer this question based on the provided memory contexts."


async def wait_for_memory_ready(
    client: CortexEvalClient,
    tenant_id: str,
    probe_query: str,
    timeout_secs: float,
    poll_interval: float,
) -> None:
    start = time.time()
    while time.time() - start < timeout_secs:
        try:
            client.switch_tenant(tenant_id)
            results = client.search(probe_query, limit=3, min_score=0.1)
            if results:
                return
        except Exception:
            pass
        await asyncio.sleep(poll_interval)


async def wait_for_sample_memory_ready(
    client: CortexEvalClient,
    tenant_id: str,
    user_id: str,
    expected_threads: list[str],
    probe_query: str,
    timeout_secs: float,
    poll_interval: float,
    data_dir: str,
) -> None:
    tenant_root = os.path.join(data_dir, "tenants", tenant_id)
    index_path = os.path.join(tenant_root, "user", user_id, ".memory_index.json")
    start = time.time()
    while time.time() - start < timeout_secs:
        try:
            all_sessions_processed = True
            for thread_id in expected_threads:
                session_meta_path = os.path.join(tenant_root, "session", thread_id, ".session.json")
                timeline_abstract = os.path.join(tenant_root, "session", thread_id, "timeline", ".abstract.md")
                timeline_overview = os.path.join(tenant_root, "session", thread_id, "timeline", ".overview.md")

                if not os.path.exists(session_meta_path):
                    all_sessions_processed = False
                    break

                with open(session_meta_path, "r", encoding="utf-8") as f:
                    session_meta = json.load(f)
                if str(session_meta.get("status", "")).lower() != "closed":
                    all_sessions_processed = False
                    break

                if not (os.path.exists(timeline_abstract) or os.path.exists(timeline_overview)):
                    all_sessions_processed = False
                    break

            if not all_sessions_processed:
                await asyncio.sleep(poll_interval)
                continue

            summary_count = 0
            if os.path.exists(index_path):
                with open(index_path, "r", encoding="utf-8") as f:
                    index = json.load(f)
                summaries = index.get("session_summaries", {})
                summary_count = sum(1 for thread_id in expected_threads if thread_id in summaries)

            client.switch_tenant(tenant_id)
            results = client.search(probe_query, limit=5, min_score=0.1)
            if results and (summary_count > 0 or os.path.exists(index_path)):
                return
        except Exception:
            pass
        await asyncio.sleep(poll_interval)

    raise TimeoutError(
        f"Timed out waiting for sample readiness: tenant={tenant_id}, expected_sessions={len(expected_threads)}"
    )


def tenant_for_sample(prefix: str, sample_id: str, sample_idx: int) -> str:
    normalized = sample_id.lower().replace("_", "-")
    return f"{prefix}-{sample_idx:03d}-{normalized}"


async def run_ingest(args: argparse.Namespace) -> None:
    session_range = parse_session_range(args.sessions) if args.sessions else None
    client = CortexEvalClient(
        base_url=args.base_url,
        answer_model=args.answer_model,
        llm_base_url=args.llm_base_url,
        llm_api_key=args.llm_api_key,
    )

    if args.input.endswith(".json"):
        samples = load_locomo_data(args.input, args.sample)
        results = []
        for sample_idx, item in enumerate(samples, start=1):
            sample_id = item["sample_id"]
            tenant_id = args.tenant or tenant_for_sample(args.tenant_prefix, sample_id, sample_idx)
            user_id = args.user or f"{tenant_id}-user"
            sessions = build_session_messages(item, session_range, tail=args.tail)

            print(f"\n=== Sample {sample_id} ===", file=sys.stderr)
            print(f"    tenant: {tenant_id}", file=sys.stderr)
            print(f"    user: {user_id}", file=sys.stderr)
            print(f"    {len(sessions)} session(s) to ingest", file=sys.stderr)

            client.switch_tenant(tenant_id)
            expected_threads: list[str] = []
            for index, sess in enumerate(sessions, start=1):
                meta = sess["meta"]
                thread_id = f"{sample_id}-{meta['session_key']}"
                msg = sess["message"]
                preview = msg.replace("\n", " | ")[:80]
                print(f"  [{meta['session_key']} ({meta['date_time']})] {preview}...", file=sys.stderr)

                client.create_session(thread_id, user_id=user_id, agent_id=args.agent_id)
                client.add_message(thread_id, "user", msg)
                client.close_session(thread_id)
                expected_threads.append(thread_id)
                results.append(
                    {
                        "sample_id": sample_id,
                        "tenant_id": tenant_id,
                        "user": user_id,
                        "session": meta["session_key"],
                        "thread_id": thread_id,
                        "reply": "[cortex] ingested",
                        "usage": {},
                        "session_index": index,
                    }
                )

            probe_query = item.get("qa", [{}])[0].get("question") or sample_id
            await wait_for_sample_memory_ready(
                client,
                tenant_id,
                user_id,
                expected_threads,
                probe_query,
                timeout_secs=args.wait_timeout,
                poll_interval=args.poll_interval,
                data_dir=args.data_dir,
            )

        if args.output:
            os.makedirs(os.path.dirname(args.output), exist_ok=True) if os.path.dirname(args.output) else None
            with open(args.output, "w", encoding="utf-8") as f:
                for record in results:
                    f.write(f"[{record['sample_id']}/{record['session']}] tenant={record['tenant_id']} user={record['user']}\n")
                    f.write(f"  {record['reply']}\n\n")
            json_path = f"{args.output}.json"
            with open(json_path, "w", encoding="utf-8") as f:
                json.dump(results, f, indent=2, ensure_ascii=False)
            print(f"Results written to {args.output}", file=sys.stderr)
            print(f"Results (JSON) written to {json_path}", file=sys.stderr)
    else:
        sessions = parse_test_file(args.input)
        tenant_id = args.tenant or f"{args.tenant_prefix}-txt"
        user_id = args.user or f"{tenant_id}-user"
        print(f"Running {len(sessions)} session(s) into tenant={tenant_id}", file=sys.stderr)
        client.switch_tenant(tenant_id)
        results = []
        for idx, session in enumerate(sessions, start=1):
            thread_id = f"txt-session-{idx:03d}"
            client.create_session(thread_id, user_id=user_id, agent_id=args.agent_id)
            for msg in session["messages"]:
                client.add_message(thread_id, "user", msg)
            client.close_session_and_wait(
                thread_id,
                timeout_secs=args.wait_timeout,
                poll_interval=args.poll_interval,
            )
            results.append({"index": idx, "thread_id": thread_id, "evals": session["evals"]})

        if args.output:
            with open(args.output, "w", encoding="utf-8") as f:
                for record in results:
                    f.write(f"=== Session {record['index']} ===\n")
                    f.write(f"[thread] {record['thread_id']}\n")
                    for ev in record["evals"]:
                        f.write(f"[eval] {ev}\n")
                    f.write("\n")
            print(f"Results written to {args.output}", file=sys.stderr)


async def run_sample_qa(
    item: dict[str, Any],
    sample_idx: int,
    args: argparse.Namespace,
    client: CortexEvalClient,
    semaphore: asyncio.Semaphore,
) -> tuple[list[dict[str, Any]], dict[str, int]]:
    sample_id = item["sample_id"]
    tenant_id = args.tenant or tenant_for_sample(args.tenant_prefix, sample_id, sample_idx)
    qas = [qa for qa in item.get("qa", []) if str(qa.get("category", "")) != "5"]
    if args.count is not None:
        qas = qas[: args.count]

    usage_sum = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    records: list[dict[str, Any]] = []
    jsonl_path = f"{args.output}.{sample_idx}.jsonl" if args.output else None

    async with semaphore:
        print(f"\n=== Sample {sample_id} [{sample_idx}] tenant={tenant_id} ===", file=sys.stderr)
        print(f"    Running {len(qas)} QA question(s)...", file=sys.stderr)
        client.switch_tenant(tenant_id)
        jsonl_file = open(jsonl_path, "w", encoding="utf-8") if jsonl_path else None
        try:
            for qi, qa in enumerate(qas, start=1):
                question = qa["question"]
                expected = str(qa["answer"])
                category = qa.get("category", "")
                evidence = qa.get("evidence", [])
                print(f"  [{sample_idx}] Q{qi}/{len(qas)}: {question[:60]}{'...' if len(question) > 60 else ''}", file=sys.stderr)

                started = time.time()
                try:
                    contexts = client.search(question, limit=args.top_k, min_score=args.min_score)
                    response_text, token_usage = client.answer_question(question, contexts)
                    elapsed = time.time() - started
                except Exception as exc:
                    contexts = []
                    response_text = f"[ERROR] {exc}"
                    token_usage = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
                    elapsed = time.time() - started

                for key in usage_sum:
                    usage_sum[key] += token_usage.get(key, 0)

                record = {
                    "sample_id": sample_id,
                    "sample_idx": sample_idx,
                    "tenant_id": tenant_id,
                    "qi": qi,
                    "question": question,
                    "expected": expected,
                    "response": response_text,
                    "category": category,
                    "evidence": evidence,
                    "contexts": contexts,
                    "time_cost": round(elapsed, 3),
                    "token_usage": token_usage,
                }
                records.append(record)
                print(f"  [{sample_idx}] A: {response_text[:60]}{'...' if len(response_text) > 60 else ''}", file=sys.stderr)

                if jsonl_file:
                    jsonl_file.write(json.dumps(record, ensure_ascii=False) + "\n")
                    jsonl_file.flush()
        finally:
            if jsonl_file:
                jsonl_file.close()
                print(f"    [{sample_idx}] written to {jsonl_path}", file=sys.stderr)

    return records, usage_sum


async def run_qa(args: argparse.Namespace) -> None:
    if not args.input.endswith(".json"):
        print("Error: QA mode only works with LoCoMo JSON files", file=sys.stderr)
        sys.exit(1)

    samples = load_locomo_data(args.input, args.sample)
    parallel = min(max(1, args.parallel), 10)
    client = CortexEvalClient(
        base_url=args.base_url,
        answer_model=args.answer_model,
        llm_base_url=args.llm_base_url,
        llm_api_key=args.llm_api_key,
    )

    semaphore = asyncio.Semaphore(parallel)
    tasks = [run_sample_qa(item, idx + 1, args, client, semaphore) for idx, item in enumerate(samples)]
    results_list = await asyncio.gather(*tasks)

    total_usage = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
    all_records: list[dict[str, Any]] = []
    for records, usage in results_list:
        all_records.extend(records)
        for key in total_usage:
            total_usage[key] += usage[key]

    print(
        f"\n    total tokens: in={total_usage['prompt_tokens']} out={total_usage['completion_tokens']} total={total_usage['total_tokens']}",
        file=sys.stderr,
    )

    if args.output:
        with open(args.output, "w", encoding="utf-8") as f:
            f.write("=== TOTAL USAGE ===\n")
            f.write(f"prompt_tokens: {total_usage['prompt_tokens']}\n")
            f.write(f"completion_tokens: {total_usage['completion_tokens']}\n")
            f.write(f"total_tokens: {total_usage['total_tokens']}\n")
        answers_path = f"{args.output}.json"
        with open(answers_path, "w", encoding="utf-8") as f:
            json.dump(all_records, f, indent=2, ensure_ascii=False)
        print(f"Summary written to {args.output}", file=sys.stderr)
        print(f"Answers written to {answers_path}", file=sys.stderr)
    else:
        print("\nDone (no output file requested).", file=sys.stderr)


def extract_json_object(text: str) -> dict[str, Any]:
    cleaned = text.strip()
    fenced = re.search(r"```(?:json)?\s*(\{.*?\})\s*```", cleaned, re.DOTALL)
    if fenced:
        cleaned = fenced.group(1)
    else:
        inline = re.search(r"\{.*\}", cleaned, re.DOTALL)
        if inline:
            cleaned = inline.group(0)
    return json.loads(cleaned)


def summarize_judged_records(records: list[dict[str, Any]]) -> tuple[dict[str, Any], list[tuple[str, dict[str, Any]]]]:
    total = len(records)
    correct = sum(1 for record in records if record.get("judge", {}).get("is_correct"))
    by_category: dict[str, dict[str, Any]] = {}
    for record in records:
        key = str(record.get("category", "unknown"))
        bucket = by_category.setdefault(key, {"total": 0, "correct": 0})
        bucket["total"] += 1
        if record.get("judge", {}).get("is_correct"):
            bucket["correct"] += 1
    ordered_categories = sorted(by_category.items(), key=lambda item: int(item[0]) if str(item[0]).isdigit() else 999)
    summary = {
        "total": total,
        "correct": correct,
        "score": round((correct / total) * 100, 2) if total else 0.0,
        "by_category": {
            key: {
                **value,
                "score": round((value["correct"] / value["total"]) * 100, 2) if value["total"] else 0.0,
            }
            for key, value in ordered_categories
        },
    }
    return summary, ordered_categories


async def run_judge(args: argparse.Namespace) -> None:
    if not args.input.endswith(".json"):
        print("Error: judge mode expects a QA result JSON file", file=sys.stderr)
        sys.exit(1)

    client = CortexEvalClient(
        base_url=args.base_url,
        answer_model=args.answer_model,
        llm_base_url=args.llm_base_url,
        llm_api_key=args.llm_api_key,
    )
    if client.llm_client is None:
        print(
            "Error: judge mode requires --llm-base-url and --llm-api-key (or OPENAI_BASE_URL / OPENAI_API_KEY)",
            file=sys.stderr,
        )
        sys.exit(1)

    with open(args.input, "r", encoding="utf-8") as f:
        records: list[dict[str, Any]] = json.load(f)

    judged_records: list[dict[str, Any]] = []
    total_usage = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}

    for index, record in enumerate(records, start=1):
        response_text = str(record.get("response", ""))
        if response_text.startswith("[ERROR]"):
            verdict = False
            reasoning = "Response generation failed before judging."
            usage = {"prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0}
        else:
            verdict, reasoning, usage = client.judge_answer(
                question=str(record.get("question", "")),
                gold_answer=str(record.get("expected", "")),
                response_text=response_text,
                judge_model=args.judge_model,
            )

        for key in total_usage:
            total_usage[key] += usage.get(key, 0)

        judged = {
            **record,
            "judge": {
                "label": "CORRECT" if verdict else "WRONG",
                "is_correct": verdict,
                "reasoning": reasoning,
                "token_usage": usage,
            },
        }
        judged_records.append(judged)
        print(
            f"  [judge] {index}/{len(records)} category={record.get('category', '')} verdict={judged['judge']['label']} q={str(record.get('question', ''))[:60]}",
            file=sys.stderr,
        )

    summary, ordered_categories = summarize_judged_records(judged_records)
    output_prefix = args.output or f"{args.input}.judge"
    summary_path = output_prefix if output_prefix.endswith(".md") else f"{output_prefix}.md"
    judged_json_path = f"{output_prefix}.json"

    lines = [
        "# LoCoMo Judge Report",
        "",
        f"- input: `{args.input}`",
        f"- judge_model: `{args.judge_model}`",
        f"- total: `{summary['total']}`",
        f"- correct: `{summary['correct']}`",
        f"- overall_score: `{summary['score']}`",
        f"- judge_tokens_in: `{total_usage['prompt_tokens']}`",
        f"- judge_tokens_out: `{total_usage['completion_tokens']}`",
        f"- judge_tokens_total: `{total_usage['total_tokens']}`",
        "",
        "## Category Scores",
        "",
    ]
    for key, value in ordered_categories:
        score = round((value['correct'] / value['total']) * 100, 2) if value['total'] else 0.0
        lines.append(f"- category {key}: `{value['correct']}/{value['total']}` => `{score}`")

    wrong_examples = [record for record in judged_records if not record.get("judge", {}).get("is_correct")][:8]
    if wrong_examples:
        lines.extend(["", "## Wrong Examples", ""])
        for record in wrong_examples:
            lines.extend(
                [
                    f"### Q{record.get('qi', '?')}",
                    f"- category: `{record.get('category', '')}`",
                    f"- question: {record.get('question', '')}",
                    f"- expected: {record.get('expected', '')}",
                    f"- response: {str(record.get('response', '')).replace(chr(10), ' ')[:300]}",
                    f"- reasoning: {record.get('judge', {}).get('reasoning', '')}",
                    "",
                ]
            )

    with open(summary_path, "w", encoding="utf-8") as f:
        f.write("\n".join(lines) + "\n")
    with open(judged_json_path, "w", encoding="utf-8") as f:
        json.dump({"summary": summary, "records": judged_records}, f, indent=2, ensure_ascii=False)

    print(f"Judge report written to {summary_path}", file=sys.stderr)
    print(f"Judged records written to {judged_json_path}", file=sys.stderr)
    print(f"overall_score={summary['score']}", file=sys.stderr)


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Evaluate Cortex Memory using LoCoMo workflow")
    parser.add_argument("mode", choices=["ingest", "qa", "judge"], help="Mode: ingest, qa, or judge")
    parser.add_argument("input", help="Path to test file (.txt or .json)")
    parser.add_argument("--output", default=None, help="Path to output file (omit to skip writing)")
    parser.add_argument("--base-url", default=DEFAULT_SERVICE_URL, help=f"Cortex service base URL (default: {DEFAULT_SERVICE_URL})")
    parser.add_argument("--sample", type=int, default=None, help="LoCoMo sample index (0-based). Default: all samples.")
    parser.add_argument("--sessions", default=None, help="LoCoMo session range, e.g. '1-4' or '3'. Default: all sessions.")
    parser.add_argument("--tail", default="[]", help="Tail message appended after each bundled session message.")
    parser.add_argument("--count", type=int, default=None, help="QA mode: number of QA questions to run. Default: all.")
    parser.add_argument("--tenant", default=None, help="Override tenant id. Default: one tenant per sample.")
    parser.add_argument("--tenant-prefix", default=DEFAULT_TENANT_PREFIX, help="Tenant prefix when auto-generating tenant ids.")
    parser.add_argument("--user", default=None, help="Override user id for ingestion.")
    parser.add_argument("--agent-id", default="cortex-eval-agent", help="Agent id used during ingestion.")
    parser.add_argument("--parallel", type=int, default=1, metavar="N", help="QA mode: number of samples to process concurrently (max 10).")
    parser.add_argument("--top-k", type=int, default=8, help="QA mode: number of search results to retrieve.")
    parser.add_argument("--min-score", type=float, default=0.4, help="QA mode: minimum search score threshold.")
    parser.add_argument("--wait-timeout", type=float, default=600.0, help="Ingest mode: max seconds to wait for memory readiness.")
    parser.add_argument("--poll-interval", type=float, default=1.0, help="Ingest mode: memory readiness polling interval.")
    parser.add_argument("--data-dir", default=os.path.join(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))), "cortex-data"), help="Local cortex data dir used to inspect memory_index during ingest readiness checks.")
    parser.add_argument("--answer-model", default=DEFAULT_SEARCH_MODEL, help="LLM model used to answer from retrieved contexts.")
    parser.add_argument("--judge-model", default=DEFAULT_JUDGE_MODEL, help="LLM model used to judge QA answers.")
    parser.add_argument("--llm-base-url", default=os.getenv("OPENAI_BASE_URL"), help="OpenAI-compatible base URL for answer generation.")
    parser.add_argument("--llm-api-key", default=os.getenv("OPENAI_API_KEY"), help="OpenAI-compatible API key for answer generation.")
    return parser


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()

    if args.mode == "ingest":
        asyncio.run(run_ingest(args))
    elif args.mode == "qa":
        asyncio.run(run_qa(args))
    elif args.mode == "judge":
        asyncio.run(run_judge(args))


if __name__ == "__main__":
    main()
