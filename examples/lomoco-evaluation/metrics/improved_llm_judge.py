"""
Improved LLM Judge for Memory System Evaluation

This module provides a more robust LLM judge for evaluating memory system answers,
with improved prompt engineering for consistency and reliability.
"""

import json
from typing import Tuple
from openai import OpenAI
import httpx


class ImprovedLLMJudge:
    """Improved LLM Judge with consistent evaluation criteria"""
    
    # Improved, more specific prompt with clear criteria
    ACCURACY_PROMPT_V2 = """
You are an expert evaluator for memory systems. Your task is to evaluate whether a generated answer is correct based on retrieved memories.

# SCORING CRITERIA

Score the answer on a scale of 0-5 with the following rubric:

**5 - Perfect Match**
- The generated answer exactly matches the gold standard answer
- All key information is present and accurate
- No incorrect information
- Example: Gold="bears, wolves, and bison", Generated="bears, wolves, and bison"

**4 - Nearly Perfect**
- The generated answer contains all key information from the gold answer
- Minor differences in phrasing or formatting
- No incorrect information
- Example: Gold="bears, wolves, and bison", Generated="bears, wolves, bison"

**3 - Correct with Minor Issues**
- The generated answer captures the main point/topic correctly
- Missing some details present in gold answer
- May include slightly extraneous but not incorrect information
- Example: Gold="bears, wolves, and bison", Generated="bears and wolves"

**2 - Partially Correct**
- The generated answer touches on the correct topic but misses key details
- Contains some inaccuracies but directionally correct
- Example: Gold="Yellowstone National Park", Generated="a national park"

**1 - Mostly Incorrect**
- The generated answer is related to the topic but largely incorrect
- Contains significant misinformation
- Example: Gold="Yellowstone National Park", Generated="Yosemite"

**0 - Completely Incorrect**
- The generated answer is completely wrong or unrelated
- Contains only hallucinations or irrelevant information
- Example: Gold="Yellowstone National Park", Generated="I don't know"

# SPECIAL RULES

1. **Time References**: For time-based answers, be lenient with format but strict on correctness.
   - "May 7th" = "7 May" = Score 5
   - "last week" = "March 15" (when conversation was March 8) = Score 0 (relative times not specific)
   - "2024" = "last year" (when current is 2025) = Score 4 (close but not exact)

2. **Quantity References**: Be exact with numbers when specified.
   - "8 hours" = "8 hours" = Score 5
   - "about 8 hours" = "8 hours" = Score 4 (imprecise)
   - "7-8 hours" = "8 hours" = Score 3 (range, not exact)

3. **List Answers**: All items must be present for Score 5.
   - Gold: "bears, wolves, and bison"
   - Gen: "bears and wolves" = Score 3 (missing item)

4. **Semantic Equivalence**: Different phrasing with same meaning is acceptable.
   - Gold: "He went hiking" 
   - Gen: "He went on a hike" = Score 5

5. **No Credit for**: General statements like "I don't know", "It depends", or vague answers unless gold answer is also vague.

# EVALUATION TASK

Question: {question}

Gold Answer: {gold_answer}

Generated Answer: {generated_answer}

# OUTPUT FORMAT

Provide your evaluation in the following JSON format:

```json
{{
  "score": <integer 0-5>,
  "explanation": "<brief explanation of your reasoning (1-2 sentences)>",
  "confidence": <high/medium/low>
}}
```

Think step-by-step:
1. What is the key information in the gold answer?
2. Does the generated answer contain this key information?
3. Are there any inaccuracies or missing details?
4. Apply the scoring rubric.
5. Output the JSON result.
"""

    def __init__(self, config_path: str):
        """Initialize the LLM judge"""
        import toml
        from pathlib import Path
        
        config = toml.load(config_path)
        
        self.client = OpenAI(
            api_key=config['llm']['api_key'],
            base_url=config['llm']['api_base_url'],
            http_client=httpx.Client(verify=False)
        )
        self.model = config['llm'].get('model_efficient', 'gpt-4o-mini')
    
    def evaluate(self, question: str, gold_answer: str, generated_answer: str, 
                max_retries: int = 2) -> Tuple[int, str, float]:
        """
        Evaluate a generated answer against the gold answer
        
        Args:
            question: The question asked
            gold_answer: The ground truth answer
            generated_answer: The answer to evaluate
            max_retries: Number of retry attempts
            
        Returns:
            Tuple of (score: int 0-5, explanation: str, confidence: float 0-1)
        """
        prompt = self.ACCURACY_PROMPT_V2.format(
            question=question,
            gold_answer=gold_answer,
            generated_answer=generated_answer
        )
        
        for attempt in range(max_retries):
            try:
                response = self.client.chat.completions.create(
                    model=self.model,
                    messages=[
                        {
                            "role": "system", 
                            "content": "You are an expert evaluator. Always respond with valid JSON."
                        },
                        {
                            "role": "user", 
                            "content": prompt
                        }
                    ],
                    response_format={"type": "json_object"},
                    temperature=0.0,
                )
                
                content = response.choices[0].message.content
                if content is None:
                    raise ValueError("Empty response from LLM")
                result = json.loads(content)
                
                score = int(result.get("score", 0))
                explanation = result.get("explanation", "")
                confidence_str = result.get("confidence", "medium")
                
                confidence_map = {"high": 0.9, "medium": 0.7, "low": 0.5}
                confidence = confidence_map.get(confidence_str, 0.7)
                
                return min(max(score, 0), 5), explanation, confidence
                
            except json.JSONDecodeError as e:
                if attempt < max_retries - 1:
                    continue
                return 0, f"JSON parsing error: {e}", 0.0
            except Exception as e:
                if attempt < max_retries - 1:
                    continue
                return 0, f"Evaluation error: {e}", 0.0
        
        return 0, "Max retries exceeded", 0.0
    
    def evaluate_binary(self, question: str, gold_answer: str, 
                      generated_answer: str, max_retries: int = 2) -> int:
        """
        Evaluate as binary (CORRECT/WRONG) for backward compatibility
        
        Args:
            question: The question asked
            gold_answer: The ground truth answer
            generated_answer: The answer to evaluate
            max_retries: Number of retry attempts
            
        Returns:
            1 if correct (score >= 3), 0 if incorrect (score < 3)
        """
        score, _, confidence = self.evaluate(question, gold_answer, generated_answer, max_retries)
        
        # Consider scores >= 3 as correct (contains correct information)
        return 1 if score >= 3 and confidence >= 0.5 else 0


def get_llm_judge(config_path: str) -> ImprovedLLMJudge:
    """Get an instance of the LLM judge"""
    return ImprovedLLMJudge(config_path)


if __name__ == "__main__":
    import argparse
    from pathlib import Path
    
    parser = argparse.ArgumentParser(description="Test LLM judge")
    parser.add_argument("--config", default="config.toml", help="Path to config file")
    
    args = parser.parse_args()
    
    judge = get_llm_judge(args.config)
    
    test_cases = [
        ("Where did Bob go hiking?", "Yellowstone National Park", "Yellowstone National Park"),
        ("Where did Bob go hiking?", "Yellowstone National Park", "He went to Yellowstone"),
        ("Where did Bob go hiking?", "Yellowstone National Park", "a national park"),
        ("How long did it take?", "8 hours", "about 8 hours"),
        ("How long did it take?", "8 hours", "7-8 hours"),
        ("How long did it take?", "8 hours", "4 hours"),
    ]
    
    for question, gold, generated in test_cases:
        score, explanation, confidence = judge.evaluate(question, gold, generated)
        print(f"\nQuestion: {question}")
        print(f"Gold: {gold}")
        print(f"Generated: {generated}")
        print(f"Score: {score}/5 (confidence: {confidence})")
        print(f"Explanation: {explanation}")
