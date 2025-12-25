import json
import random
from datetime import datetime, timedelta

# Base templates for different conversation types
conversation_templates = [
    {
        "topic": "hobbies",
        "speakers": ["Alex", "Jordan"],
        "content": [
            {"speaker": "Alex", "text": "Hey Jordan! Have you been working on that painting you mentioned?"},
            {"speaker": "Jordan", "text": "Yes! I actually finished it yesterday. It's a landscape of the mountains near my hometown."},
            {"speaker": "Alex", "text": "That sounds beautiful! What medium did you use?"},
            {"speaker": "Jordan", "text": "I used oil paints on canvas. It took me about two weeks to complete."},
            {"speaker": "Alex", "text": "Two weeks of dedication! Did you use any specific technique?"},
            {"speaker": "Jordan", "text": "I used the glazing technique for the sky. It creates really nice depth."},
        ],
        "questions": [
            {"question": "What did Jordan paint?", "answer": "a landscape of the mountains", "category": "1"},
            {"question": "How long did it take Jordan to finish the painting?", "answer": "two weeks", "category": "3"},
            {"question": "What technique did Jordan use for the sky?", "answer": "glazing technique", "category": "1"},
        ]
    },
    {
        "topic": "travel",
        "speakers": ["Maya", "Liam"],
        "content": [
            {"speaker": "Maya", "text": "Liam, I heard you're going to Japan next month!"},
            {"speaker": "Liam", "text": "Yes! I'm so excited. I'll be staying in Tokyo and Kyoto."},
            {"speaker": "Maya", "text": "How long is your trip?"},
            {"speaker": "Liam", "text": "I'll be there for two weeks, from March 15th to March 29th."},
            {"speaker": "Maya", "text": "That's a great length! What are you most looking forward to?"},
            {"speaker": "Liam", "text": "I can't wait to visit the cherry blossoms in Kyoto. It should be perfect timing."},
        ],
        "questions": [
            {"question": "Where is Liam going for his trip?", "answer": "Japan (Tokyo and Kyoto)", "category": "1"},
            {"question": "How long is Liam staying in Japan?", "answer": "two weeks", "category": "3"},
            {"question": "What is Liam most excited to see in Kyoto?", "answer": "cherry blossoms", "category": "1"},
        ]
    },
    {
        "topic": "work_project",
        "speakers": ["Sam", "Taylor"],
        "content": [
            {"speaker": "Sam", "text": "Taylor, how's the project coming along?"},
            {"speaker": "Taylor", "text": "It's going well! We've completed the first phase ahead of schedule."},
            {"speaker": "Sam", "text": "That's great news! When did you finish it?"},
            {"speaker": "Taylor", "text": "We finished last Friday, two days before the deadline."},
            {"speaker": "Sam", "text": "Excellent! What's next for the team?"},
            {"speaker": "Taylor", "text": "Now we're moving to the testing phase. We expect it to take about a week."},
        ],
        "questions": [
            {"question": "What phase has Taylor's team completed?", "answer": "the first phase", "category": "1"},
            {"question": "When did they finish the first phase?", "answer": "last Friday", "category": "2"},
            {"question": "How long will the testing phase take?", "answer": "about a week", "category": "3"},
        ]
    },
    {
        "topic": "food",
        "speakers": ["Chef Marco", "Emily"],
        "content": [
            {"speaker": "Emily", "text": "Chef Marco, that pasta dish was incredible! What was in the sauce?"},
            {"speaker": "Chef Marco", "text": "Thank you, Emily! The secret is fresh basil, garlic, and a touch of truffle oil."},
            {"speaker": "Emily", "text": "And what kind of pasta did you use?"},
            {"speaker": "Chef Marco", "text": "I made fresh fettuccine this morning. It makes such a difference."},
            {"speaker": "Emily", "text": "How long did it take to make the pasta from scratch?"},
            {"speaker": "Chef Marco", "text": "About 45 minutes. It's worth the effort for the texture."},
        ],
        "questions": [
            {"question": "What ingredients were in the sauce?", "answer": "fresh basil, garlic, and truffle oil", "category": "1"},
            {"question": "What type of pasta did Chef Marco use?", "answer": "fresh fettuccine", "category": "1"},
            {"question": "How long did it take to make the fresh pasta?", "answer": "45 minutes", "category": "3"},
        ]
    },
    {
        "topic": "sports",
        "speakers": ["Coach Mike", "Sarah"],
        "content": [
            {"speaker": "Sarah", "text": "Coach Mike, what time is our practice tomorrow?"},
            {"speaker": "Coach Mike", "text": "We're starting at 6 AM, Sarah. I want everyone there by 5:45 for warm-ups."},
            {"speaker": "Sarah", "text": "Got it! What are we focusing on tomorrow?"},
            {"speaker": "Coach Mike", "text": "Defense drills and stamina training. We have a big game on Saturday."},
            {"speaker": "Sarah", "text": "Who are we playing against?"},
            {"speaker": "Coach Mike", "text": "The Tigers. They beat us last year, so we need to be prepared."},
        ],
        "questions": [
            {"question": "What time does practice start tomorrow?", "answer": "6 AM", "category": "3"},
            {"question": "When should players arrive for warm-ups?", "answer": "5:45 AM", "category": "3"},
            {"question": "Who are they playing against on Saturday?", "answer": "the Tigers", "category": "1"},
        ]
    },
    {
        "topic": "music",
        "speakers": ["DJ Luna", "Kevin"],
        "content": [
            {"speaker": "Kevin", "text": "DJ Luna, that new track you played was amazing!"},
            {"speaker": "DJ Luna", "text": "Thanks Kevin! I've been working on it for months."},
            {"speaker": "Kevin", "text": "What genre would you call it?"},
            {"speaker": "DJ Luna", "text": "It's a fusion of deep house and lo-fi beats. Really unique sound."},
            {"speaker": "Kevin", "text": "How did you get the idea for this fusion?"},
            {"speaker": "DJ Luna", "text": "I was inspired by my trip to Berlin last summer. The music scene there is incredible."},
        ],
        "questions": [
            {"question": "What is the genre of DJ Luna's new track?", "answer": "fusion of deep house and lo-fi beats", "category": "1"},
            {"question": "Where did DJ Luna get the inspiration for the fusion?", "answer": "Berlin", "category": "1"},
            {"question": "When did DJ Luna visit Berlin?", "answer": "last summer", "category": "2"},
        ]
    },
    {
        "topic": "technology",
        "speakers": ["Dr. Chen", "Tom"],
        "content": [
            {"speaker": "Tom", "text": "Dr. Chen, I heard your paper on quantum computing was accepted!"},
            {"speaker": "Dr. Chen", "text": "Yes Tom! It's being published in the Science journal next month."},
            {"speaker": "Tom", "text": "Congratulations! What's the main breakthrough in your research?"},
            {"speaker": "Dr. Chen", "text": "We've developed a new error correction algorithm that reduces decoherence by 40%."},
            {"speaker": "Tom", "text": "That's significant! How will this impact the field?"},
            {"speaker": "Dr. Chen", "text": "It could make practical quantum computers feasible within the next 5-10 years."},
        ],
        "questions": [
            {"question": "Which journal is Dr. Chen's paper being published in?", "answer": "Science journal", "category": "1"},
            {"question": "What is the main breakthrough in Dr. Chen's research?", "answer": "new error correction algorithm", "category": "1"},
            {"question": "How much does the algorithm reduce decoherence?", "answer": "40%", "category": "3"},
        ]
    },
    {
        "topic": "fitness",
        "speakers": ["Trainer Alex", "Maria"],
        "content": [
            {"speaker": "Maria", "text": "Trainer Alex, I want to increase my bench press. What should I do?"},
            {"speaker": "Trainer Alex", "text": "Let's start with your current weight. How much are you benching now?"},
            {"speaker": "Maria", "text": "I'm at 135 pounds currently."},
            {"speaker": "Trainer Alex", "text": "Good base! We'll use progressive overload. Add 5 pounds every week."},
            {"speaker": "Maria", "text": "How many sets and reps should I do?"},
            {"speaker": "Trainer Alex", "text": "For strength, do 5 sets of 5 reps. Rest 3-4 minutes between sets."},
        ],
        "questions": [
            {"question": "How much is Maria currently bench pressing?", "answer": "135 pounds", "category": "3"},
            {"question": "How much should Maria add each week?", "answer": "5 pounds", "category": "3"},
            {"question": "How many sets and reps for strength training?", "answer": "5 sets of 5 reps", "category": "1"},
        ]
    },
    {
        "topic": "books",
        "speakers": ["Bookworm Lily", "Mark"],
        "content": [
            {"speaker": "Mark", "text": "Lily, have you read any good books lately?"},
            {"speaker": "Bookworm Lily", "text": "Yes! I just finished 'The Midnight Library' by Matt Haig. It was profound."},
            {"speaker": "Mark", "text": "I've heard of it. What's it about?"},
            {"speaker": "Bookworm Lily", "text": "It explores the infinite lives we could have lived, based on different choices. Really makes you think."},
            {"speaker": "Mark", "text": "Sounds interesting! What was your favorite part?"},
            {"speaker": "Bookworm Lily", "text": "The ending was perfect - it showed that even ordinary lives can be meaningful."},
        ],
        "questions": [
            {"question": "What book did Lily just finish reading?", "answer": "The Midnight Library by Matt Haig", "category": "1"},
            {"question": "Who is the author of the book?", "answer": "Matt Haig", "category": "1"},
            {"question": "What is the book about?", "answer": "infinite lives based on different choices", "category": "2"},
        ]
    },
    {
        "topic": "gardening",
        "speakers": ["Gardener Rose", "Ben"],
        "content": [
            {"speaker": "Ben", "text": "Rose, your garden looks amazing this year! What's your secret?"},
            {"speaker": "Gardener Rose", "text": "Thanks Ben! I switched to organic compost this year. Makes a huge difference."},
            {"speaker": "Ben", "text": "What kind of compost do you use?"},
            {"speaker": "Gardener Rose", "text": "I make my own from kitchen scraps and yard waste. It takes about 6 months to mature."},
            {"speaker": "Ben", "text": "Do you add anything else to the soil?"},
            {"speaker": "Gardener Rose", "text": "Yes, I also use worm castings. They're rich in nutrients and improve soil structure."},
        ],
        "questions": [
            {"question": "What change did Rose make to her garden this year?", "answer": "switched to organic compost", "category": "1"},
            {"question": "How long does it take Rose's compost to mature?", "answer": "about 6 months", "category": "3"},
            {"question": "What else does Rose add to the soil besides compost?", "answer": "worm castings", "category": "1"},
        ]
    }
]

def generate_conversation(idx, template):
    """Generate a conversation from template with random variations"""
    base_date = datetime(2024, random.randint(1, 12), random.randint(1, 28))
    
    conversation_data = {
        "conversation": {
            "speaker_a": template["speakers"][0],
            "speaker_b": template["speakers"][1],
        },
        "qa": []
    }
    
    # Create 2-3 sessions per conversation
    num_sessions = random.randint(2, 3)
    for session_idx in range(num_sessions):
        session_key = f"session_{session_idx + 1}"
        
        # Create variations of the base content
        session_content = []
        for item in template["content"]:
            session_content.append({
                "speaker": item["speaker"],
                "text": item["text"]
            })
        
        # Add some filler conversation
        filler_options = [
            {"speaker": template["speakers"][0], "text": "That's really interesting!"},
            {"speaker": template["speakers"][1], "text": "Tell me more about it."},
            {"speaker": template["speakers"][0], "text": "I didn't know that!"},
            {"speaker": template["speakers"][1], "text": "Thanks for sharing!"},
        ]
        
        if session_idx == 0:
            session_date = base_date
        else:
            session_date = base_date + timedelta(days=session_idx * random.randint(2, 7))
        
        conversation_data["conversation"][session_key] = session_content
        conversation_data["conversation"][f"{session_key}_date_time"] = session_date.strftime("%Y-%m-%d %H:%M:%S")
    
    # Generate questions with evidence
    for q in template["questions"]:
        evidence_template = f"{template['speakers'][1 if q['question'].split()[0].lower() == 'you' or 'what' in q['question'].lower() or 'how' in q['question'].lower() else 0]} said"
        
        # Find relevant content for evidence
        for item in template["content"]:
            if any(word.lower() in item["text"].lower() for word in q["answer"].lower().split() if len(word) > 2):
                evidence_template = f"{item['speaker']} said '{item['text']}'"
                break
        
        conversation_data["qa"].append({
            "question": q["question"],
            "answer": q["answer"],
            "category": q["category"],
            "evidence": [evidence_template],
            "adversarial_answer": f"I don't have that information."
        })
    
    return conversation_data

def generate_enhanced_dataset(num_conversations=50, output_file="dataset/locomo50.json"):
    """Generate an enhanced LOCOMO-style dataset"""
    conversations = []
    
    # Use all templates multiple times with variations
    template_cycle = 0
    for i in range(num_conversations):
        template = conversation_templates[template_cycle % len(conversation_templates)]
        
        # Add variations for repeated templates
        if template_cycle >= len(conversation_templates):
            # Slightly modify speaker names for variety
            variations = ["Alex", "Jordan", "Maya", "Liam", "Sam", "Taylor", "Chef Marco", "Emily", "Coach Mike", "Sarah"]
            template = template.copy()
            offset = (template_cycle // len(conversation_templates)) * 2
            if offset + 1 < len(variations):
                template["speakers"] = [variations[offset], variations[offset + 1]]
        
        conversation = generate_conversation(i, template)
        conversations.append(conversation)
        template_cycle += 1
    
    # Save to file
    with open(output_file, 'w') as f:
        json.dump(conversations, f, indent=2)
    
    print(f"Generated {num_conversations} conversations in {output_file}")
    print(f"Total questions: {sum(len(c['qa']) for c in conversations)}")
    
    return conversations

if __name__ == "__main__":
    generate_enhanced_dataset(num_conversations=50)
    print("\n✅ Enhanced dataset generated successfully!")
    print("You can now use 'dataset/locomo50.json' for evaluation.")
