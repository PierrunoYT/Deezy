/**
 * Rate limiter to prevent overwhelming the Deezer API
 * Implements token bucket algorithm for smooth request distribution
 */

const MS_PER_SECOND = 1000;

class RateLimiter {
  private lastCallTime = 0;
  private readonly minInterval: number;
  private readonly callsPerSecond: number;
  private callCount = 0;
  // Serializes concurrent callers so only one runs at a time through the gate.
  private queue: Promise<void> = Promise.resolve();

  constructor(callsPerSecond: number) {
    if (callsPerSecond <= 0) {
      throw new Error('callsPerSecond must be positive');
    }
    
    this.callsPerSecond = callsPerSecond;
    this.minInterval = MS_PER_SECOND / callsPerSecond;
  }

  async throttle(): Promise<void> {
    // Chain each caller onto the shared queue so concurrent calls are serialized
    // and each one waits its own minimum interval rather than all reading the
    // same lastCallTime simultaneously and bypassing the rate limit.
    this.queue = this.queue.then(() => this.doThrottle());
    return this.queue;
  }

  private async doThrottle(): Promise<void> {
    const now = Date.now();
    const timeSinceLastCall = now - this.lastCallTime;
    
    if (timeSinceLastCall < this.minInterval) {
      const waitTime = this.minInterval - timeSinceLastCall;
      await this.sleep(waitTime);
      this.lastCallTime = Date.now();
    } else {
      this.lastCallTime = now;
    }

    this.callCount++;
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  reset(): void {
    this.lastCallTime = 0;
    this.callCount = 0;
    this.queue = Promise.resolve();
  }

  getCallCount(): number {
    return this.callCount;
  }

  getTimeSinceLastCall(): number {
    return Date.now() - this.lastCallTime;
  }

  canCallNow(): boolean {
    return this.getTimeSinceLastCall() >= this.minInterval;
  }

  getMinInterval(): number {
    return this.minInterval;
  }

  getCallsPerSecond(): number {
    return this.callsPerSecond;
  }
}

export const searchRateLimiter = new RateLimiter(2);
export const downloadRateLimiter = new RateLimiter(3);

