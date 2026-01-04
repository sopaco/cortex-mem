"""
Rate limiter for API calls to avoid hitting rate limits.
Implements a simple token bucket algorithm.
"""
import time
import threading
from typing import Optional


class RateLimiter:
    """
    Rate limiter using token bucket algorithm.
    Ensures API calls don't exceed specified rate per minute.
    """

    def __init__(self, max_calls_per_minute: int = 30):
        """
        Initialize rate limiter.
        
        Args:
            max_calls_per_minute: Maximum number of calls allowed per minute
        """
        self.max_calls = max_calls_per_minute
        self.interval = 60.0 / max_calls_per_minute  # seconds between calls
        self.last_call_time = 0.0
        self.lock = threading.Lock()

    def acquire(self):
        """
        Acquire permission to make an API call.
        Blocks if necessary to maintain rate limit.
        """
        with self.lock:
            current_time = time.time()
            time_since_last_call = current_time - self.last_call_time
            
            if time_since_last_call < self.interval:
                # Need to wait
                sleep_time = self.interval - time_since_last_call
                time.sleep(sleep_time)
                self.last_call_time = time.time()
            else:
                self.last_call_time = current_time

    def __enter__(self):
        """Context manager entry."""
        self.acquire()
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        return False
