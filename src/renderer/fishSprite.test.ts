import { describe, it, expect, beforeEach } from "vitest";
import {
  hasCachedSprite,
  getCachedSprite,
  evictStaleSprites,
  clearSpriteCache,
} from "./fishSprite";

describe("sprite cache management", () => {
  beforeEach(() => {
    clearSpriteCache();
  });

  it("cache starts empty", () => {
    expect(hasCachedSprite(1)).toBe(false);
    expect(getCachedSprite(1)).toBeUndefined();
  });

  it("hasCachedSprite returns false for unknown genome", () => {
    expect(hasCachedSprite(99999)).toBe(false);
  });

  it("getCachedSprite returns undefined for uncached genome", () => {
    expect(getCachedSprite(42)).toBeUndefined();
  });

  it("clearSpriteCache works without error on empty cache", () => {
    clearSpriteCache();
    expect(hasCachedSprite(1)).toBe(false);
  });

  it("evictStaleSprites works on empty cache", () => {
    const activeIds = new Set([1, 2, 3]);
    evictStaleSprites(activeIds);
    expect(hasCachedSprite(1)).toBe(false);
  });

  it("evictStaleSprites with empty active set works on empty cache", () => {
    const activeIds = new Set<number>();
    evictStaleSprites(activeIds);
    // Should not throw
  });
});
