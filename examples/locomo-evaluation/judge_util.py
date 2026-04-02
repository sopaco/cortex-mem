import asyncio
import json
import os
from typing import Any

from dotenv import load_dotenv
from openai import AsyncOpenAI


async def locomo_grader(
    llm_client: AsyncOpenAI,
    model: str,
    question: str,
    gold_answer: str,
    response: str,
) -> tuple[bool, str]:
    system_prompt = """
        You are an expert grader that determines if answers to questions match a gold standard answer.
    """

    accuracy_prompt = f"""
    Your task is to label an answer to a question as 'CORRECT' or 'WRONG'. You will be given the following data:
        (1) a question (posed by one user to another user),
        (2) a 'gold' (ground truth) answer,
        (3) a generated answer
    which you will score as CORRECT/WRONG.

    The point of the question is to ask about something one user should know about the other user based on their prior conversations.
    The gold answer will usually be a concise and short answer that includes the referenced topic, for example:
    Question: Do you remember what I got the last time I went to Hawaii?
    Gold answer: A shell necklace
    The generated answer might be much longer, but you should be generous with your grading - as long as it touches on the same topic as the gold answer, it should be counted as CORRECT.

    For time related questions, the gold answer will be a specific date, month, year, etc. The generated answer might be much longer or use relative time references (like "last Tuesday" or "next month"), but you should be generous with your grading - as long as it refers to the same date or time period as the gold answer, it should be counted as CORRECT. Even if the format differs (e.g., "May 7th" vs "7 May"), consider it CORRECT if it's the same date.

    Now it's time for the real question:
    Question: {question}
    Gold answer: {gold_answer}
    Generated answer: {response}

    First, provide a short (one sentence) explanation of your reasoning, then finish with CORRECT or WRONG.
    Do NOT include both CORRECT and WRONG in your response, or it will break the evaluation script.

    Respond with JSON only: {{"is_correct": "CORRECT" or "WRONG", "reasoning": "your explanation"}}
    """

    response_obj = await llm_client.chat.completions.create(
        model=model,
        messages=[
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": accuracy_prompt},
        ],
        temperature=0,
    )

    content = response_obj.choices[0].message.content or "{}"
    start_idx = content.find("{")
    end_idx = content.rfind("}")
    payload = (
        content[start_idx : end_idx + 1]
        if start_idx != -1 and end_idx != -1
        else content
    )
    result = json.loads(payload)
    label = str(result.get("is_correct", result.get("label", "WRONG"))).strip().lower()
    reasoning = str(result.get("reasoning", "")).strip()
    return label == "correct", reasoning


def load_answers(path: str) -> list[dict[str, Any]]:
    with open(path, "r", encoding="utf-8") as f:
        if path.endswith(".jsonl"):
            return [json.loads(line) for line in f if line.strip()]
        data = json.load(f)
        if isinstance(data, dict):
            return data.get("results", [])
        return data


async def grade_answers(
    answers: list[dict[str, Any]],
    base_url: str | None = None,
    api_key: str | None = None,
    model: str = "gpt-5-mini",
    parallel: int = 5,
) -> list[dict[str, Any]]:
    load_dotenv()
    client = AsyncOpenAI(
        base_url=base_url or os.getenv("OPENAI_BASE_URL"),
        api_key=api_key or os.getenv("OPENAI_API_KEY"),
    )
    semaphore = asyncio.Semaphore(max(1, parallel))

    async def grade_one(item: dict[str, Any]) -> dict[str, Any]:
        async with semaphore:
            is_correct, reasoning = await locomo_grader(
                client,
                model,
                item["question"],
                item["expected"],
                item["response"],
            )
            return {**item, "grade": is_correct, "reasoning": reasoning}

    return await asyncio.gather(*(grade_one(item) for item in answers))
