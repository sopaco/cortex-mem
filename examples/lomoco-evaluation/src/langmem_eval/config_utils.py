"""
Configuration utilities for LangMem evaluation.
Handles config.toml file reading and validation.
"""

import os
from pathlib import Path


def validate_config(config_path: str) -> bool:
    """Validate that config file exists and has required settings."""
    if not os.path.exists(config_path):
        print(f"Config file not found: {config_path}")
        return False
    
    try:
        with open(config_path, 'r') as f:
            content = f.read()
        
        # Check for required sections
        required_sections = ["llm"]
        missing_sections = []
        
        for section in required_sections:
            if f"[{section}]" not in content:
                missing_sections.append(section)
        
        if missing_sections:
            print(f"Missing required sections in config: {missing_sections}")
            return False
        
        # Check for required fields in each section
        import toml
        
        config_data = toml.load(config_path)
        
        # Check llm section
        if "llm" in config_data:
            llm = config_data["llm"]
            required_llm_fields = ["api_key", "api_base_url", "model_efficient"]
            missing_llm = [field for field in required_llm_fields if field not in llm]
            if missing_llm:
                print(f"Missing fields in [llm] section: {missing_llm}")
                return False
        
        return True
        
    except Exception as e:
        print(f"Error validating config: {e}")
        return False


def get_config_value(config_path: str, section: str, key: str, default=None):
    """Get a specific value from config file."""
    try:
        import toml
        config_data = toml.load(config_path)
        
        if section in config_data and key in config_data[section]:
            return config_data[section][key]
        return default
    except:
        return default


def check_openai_config(config_path: str) -> bool:
    """Check if OpenAI configuration is properly set."""
    try:
        import toml
        config_data = toml.load(config_path)
        
        # Check llm section
        if "llm" not in config_data:
            print("Missing [llm] section in config")
            return False
        
        llm = config_data["llm"]
        if "api_key" not in llm or not llm["api_key"]:
            print("OpenAI API key not set in [llm] section")
            return False
        
        if "api_base_url" not in llm or not llm["api_base_url"]:
            print("OpenAI API base URL not set in [llm] section")
            return False
        
        return True
        
    except Exception as e:
        print(f"Error checking OpenAI config: {e}")
        return False


def get_langmem_model_string(config_path: str) -> str:
    """
    Get the model string for LangMem from config.toml.
    LangMem expects model strings in format like "openai:gpt-4o-2024-11-20"
    or "custom:base_url:model_name"
    """
    try:
        import toml
        config_data = toml.load(config_path)
        
        if "llm" in config_data:
            api_base_url = config_data["llm"].get("api_base_url", "")
            model_name = config_data["llm"].get("model_efficient", "")
            
            # For custom OpenAI-compatible APIs, use the format that LangMem can understand
            # We'll construct a custom model identifier
            if api_base_url and model_name:
                # Store the base URL and model name separately, we'll use them directly
                # when creating the LangMem client
                return f"openai:{model_name}"
        
        return "openai:gpt-4o"
    except Exception as e:
        print(f"Error getting LangMem model string: {e}")
        return "openai:gpt-4o"