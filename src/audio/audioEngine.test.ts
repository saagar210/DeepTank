import { describe, it, expect } from "vitest";
import { AudioEngine } from "./audioEngine";

describe("AudioEngine (no AudioContext)", () => {
  it("creates without throwing", () => {
    const engine = new AudioEngine();
    expect(engine).toBeDefined();
  });

  it("muted defaults to false", () => {
    const engine = new AudioEngine();
    expect(engine.muted).toBe(false);
  });

  it("muted setter works without init", () => {
    const engine = new AudioEngine();
    engine.muted = true;
    expect(engine.muted).toBe(true);
    engine.muted = false;
    expect(engine.muted).toBe(false);
  });

  it("play methods are safe to call without init", () => {
    const engine = new AudioEngine();
    // These should all be no-ops without AudioContext
    expect(() => engine.playBubble()).not.toThrow();
    expect(() => engine.playBirth()).not.toThrow();
    expect(() => engine.playDeath()).not.toThrow();
    expect(() => engine.playNewSpecies()).not.toThrow();
    expect(() => engine.playExtinction()).not.toThrow();
    expect(() => engine.playFeed()).not.toThrow();
    expect(() => engine.playTap()).not.toThrow();
  });

  it("destroy is safe to call without init", () => {
    const engine = new AudioEngine();
    expect(() => engine.destroy()).not.toThrow();
  });

  it("destroy is safe to call twice", () => {
    const engine = new AudioEngine();
    engine.destroy();
    expect(() => engine.destroy()).not.toThrow();
  });

  it("setters work without init", () => {
    const engine = new AudioEngine();
    expect(() => { engine.masterVolume = 0.5; }).not.toThrow();
    expect(() => { engine.ambientEnabled = false; }).not.toThrow();
    expect(() => { engine.eventEnabled = false; }).not.toThrow();
  });


  it("clamps masterVolume to valid range", () => {
    const engine = new AudioEngine();

    engine.masterVolume = -1;
    expect(engine.masterVolume).toBe(0);

    engine.masterVolume = 2;
    expect(engine.masterVolume).toBe(1);
  });

  it("normalizes non-finite masterVolume values", () => {
    const engine = new AudioEngine();

    engine.masterVolume = Number.NaN;
    expect(engine.masterVolume).toBe(0.3);

    engine.masterVolume = Number.POSITIVE_INFINITY;
    expect(engine.masterVolume).toBe(0.3);
  });

});
