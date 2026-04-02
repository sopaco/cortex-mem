"""
Grade Cortex Memory QA responses using an LLM judge.
"""

import argparse
import asyncio
import json
import sys

from judge_util import grade_answers, load_answers


async def run(
    input_path: str,
    output_path: str | None,
    base_url: str | None,
    token: str | None,
    model: str,
    parallel: int,
) -> None:
    answers = load_answers(input_path)
    print(f"Loaded {len(answers)} answers from {input_path}", file=sys.stderr)

    graded = await grade_answers(
        answers,
        base_url=base_url,
        api_key=token,
        model=model,
        parallel=parallel,
    )

    correct = sum(1 for item in graded if item["grade"])
    total = len(graded)
    score = correct / total if total > 0 else 0.0

    print(f"\nResults: {correct}/{total} correct ({score:.2%})")

    categories: dict[str, dict[str, int]] = {}
    for item in graded:
        category = str(item.get("category", "unknown"))
        bucket = categories.setdefault(category, {"correct": 0, "total": 0})
        bucket["total"] += 1
        if item["grade"]:
            bucket["correct"] += 1

    if len(categories) > 1:
        print("\nPer-category scores:")
        for category in sorted(categories):
            stats = categories[category]
            pct = stats["correct"] / stats["total"] if stats["total"] > 0 else 0.0
            print(
                f"  Category {category}: {stats['correct']}/{stats['total']} ({pct:.2%})"
            )

    if output_path:
        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(
                {
                    "score": score,
                    "correct": correct,
                    "total": total,
                    "grades": graded,
                },
                f,
                indent=2,
                ensure_ascii=False,
            )
        print(f"\nGrades written to {output_path}", file=sys.stderr)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Grade Cortex Memory QA responses with LLM judge"
    )
    parser.add_argument("input", help="Path to answers JSON/JSONL file")
    parser.add_argument("--output", default=None, help="Path to write grades JSON")
    parser.add_argument(
        "--base-url", default=None, help="LLM API base URL (or set OPENAI_BASE_URL)"
    )
    parser.add_argument(
        "--token", default=None, help="LLM API key (or set OPENAI_API_KEY)"
    )
    parser.add_argument("--model", default="gpt-5-mini", help="Model name for grading")
    parser.add_argument(
        "--parallel", type=int, default=5, help="Parallel grading requests"
    )
    args = parser.parse_args()

    asyncio.run(
        run(
            args.input,
            args.output,
            args.base_url,
            args.token,
            args.model,
            args.parallel,
        )
    )


if __name__ == "__main__":
    main()
