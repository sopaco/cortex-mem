import argparse
import json
from collections import defaultdict

import numpy as np
from openai import OpenAI


def extract_json(text):
    """Extract JSON from text response."""
    # If already a dict, return it directly
    if isinstance(text, dict):
        return text
    
    # Try to find JSON in the text
    import re
    
    # First try to parse the entire text as JSON
    try:
        return json.loads(text)
    except (json.JSONDecodeError, TypeError):
        pass
    
    # Try to find JSON object in the text
    json_match = re.search(r'\{[^{}]*\}', text)
    if json_match:
        try:
            return json.loads(json_match.group())
        except json.JSONDecodeError:
            pass
    
    # If all else fails, return the original text
    return text


# Initialize OpenAI client from config.toml
def get_openai_client():
    """Get OpenAI client configured from config.toml"""
    import os
    from pathlib import Path
    import toml
    import httpx
    
    # Find config.toml
    config_path = None
    current_dir = Path(__file__).parent.parent
    
    if (current_dir / "config.toml").exists():
        config_path = current_dir / "config.toml"
    else:
        # Check parent directories
        for parent in current_dir.parents:
            if (parent / "config.toml").exists():
                config_path = parent / "config.toml"
                break
    
    if not config_path:
        raise FileNotFoundError("Could not find config.toml file")
    
    # Load config
    config = toml.load(config_path)
    
    # Create HTTP client with SSL verification disabled for internal APIs
    http_client = httpx.Client(verify=False)
    
    return OpenAI(
        api_key=config['llm']['api_key'],
        base_url=config['llm']['api_base_url'],
        http_client=http_client
    )


client = get_openai_client()

ACCURACY_PROMPT = """
Your task is to label an answer to a question as ’CORRECT’ or ’WRONG’. You will be given the following data:
    (1) a question (posed by one user to another user), 
    (2) a ’gold’ (ground truth) answer, 
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
Generated answer: {generated_answer}

First, provide a short (one sentence) explanation of your reasoning, then finish with CORRECT or WRONG. 
Do NOT include both CORRECT and WRONG in your response, or it will break the evaluation script.

Just return the label CORRECT or WRONG in a json format with the key as "label".
"""


def evaluate_llm_judge(question, gold_answer, generated_answer):
    """Evaluate the generated answer against the gold answer using an LLM judge."""
    import toml
    from pathlib import Path
    
    # Get model from config.toml
    config_path = None
    current_dir = Path(__file__).parent.parent
    
    if (current_dir / "config.toml").exists():
        config_path = current_dir / "config.toml"
    else:
        for parent in current_dir.parents:
            if (parent / "config.toml").exists():
                config_path = parent / "config.toml"
                break
    
    config = toml.load(config_path)
    model = config['llm'].get('model_efficient', 'gpt-4o-mini')
    
    response = client.chat.completions.create(
        model=model,
        messages=[
            {
                "role": "user",
                "content": ACCURACY_PROMPT.format(
                    question=question, gold_answer=gold_answer, generated_answer=generated_answer
                ),
            }
        ],
        response_format={"type": "json_object"},
        temperature=0.0,
    )
    
    # Add delay to avoid rate limiting
    import time
    time.sleep(0.5)
    
    # Get the response content and parse it
    content = response.choices[0].message.content
    result = extract_json(content)
    
    label = result["label"]
    return 1 if label == "CORRECT" else 0


def main():
    """Main function to evaluate RAG results using LLM judge."""
    parser = argparse.ArgumentParser(description="Evaluate RAG results using LLM judge")
    parser.add_argument(
        "--input_file",
        type=str,
        default="results/default_run_v4_k30_new_graph.json",
        help="Path to the input dataset file",
    )

    args = parser.parse_args()

    dataset_path = args.input_file
    output_path = f"results/llm_judge_{dataset_path.split('/')[-1]}"

    with open(dataset_path, "r") as f:
        data = json.load(f)

    LLM_JUDGE = defaultdict(list)
    RESULTS = defaultdict(list)

    index = 0
    for k, v in data.items():
        for x in v:
            question = x["question"]
            gold_answer = x["answer"]
            generated_answer = x["response"]
            category = x["category"]

            # Skip category 5
            if int(category) == 5:
                continue

            # Evaluate the answer
            label = evaluate_llm_judge(question, gold_answer, generated_answer)
            LLM_JUDGE[category].append(label)

            # Store the results
            RESULTS[index].append(
                {
                    "question": question,
                    "gt_answer": gold_answer,
                    "response": generated_answer,
                    "category": category,
                    "llm_label": label,
                }
            )

            # Save intermediate results
            with open(output_path, "w") as f:
                json.dump(RESULTS, f, indent=4)

            # Print current accuracy for all categories
            print("All categories accuracy:")
            for cat, results in LLM_JUDGE.items():
                if results:  # Only print if there are results for this category
                    print(f"  Category {cat}: {np.mean(results):.4f} ({sum(results)}/{len(results)})")
            print("------------------------------------------")
        index += 1

    # Save final results
    with open(output_path, "w") as f:
        json.dump(RESULTS, f, indent=4)

    # Print final summary
    print("PATH: ", dataset_path)
    print("------------------------------------------")
    for k, v in LLM_JUDGE.items():
        print(k, np.mean(v))


if __name__ == "__main__":
    main()
