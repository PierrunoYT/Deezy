/**
 * Simple rate limiter to prevent overwhelming the Deezer API
 */
class RateLimiter {
  private lastCallTime = 0;
  private minInterval: number;

  constructor(callsPerSecond: number) {
    this.minInterval = 1000 / callsPerSecond;
  }

  async throttle(): Promise<void> {
    const now = Date.now();
    const timeSinceLastCall = now - this.lastCallTime;
    
    if (timeSinceLastCall < this.minInterval) {
      const waitTime = this.minInterval - timeSinceLastCall;
      await new Promise(resolve => setTimeout(resolve, waitTime));
    }
    
    this.lastCallTime = Date.now();
  }
}

// Create rate limiters for different operations
// Search: 2 calls per second
export const searchRateLimiter = new RateLimiter(2);

// Download: 3 calls per second (handled by queue, but add extra safety)
export const downloadRateLimiter = new RateLimiter(3);

