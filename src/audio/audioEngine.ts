/**
 * Procedural audio engine using Web Audio API.
 * Generates ambient underwater sounds and event-driven effects entirely with synthesis.
 */


function normalizeVolume(value: number): number {
  if (!Number.isFinite(value)) return 0.3;
  return Math.min(1, Math.max(0, value));
}

export class AudioEngine {
  private ctx: AudioContext | null = null;
  private masterGain: GainNode | null = null;
  private ambientGain: GainNode | null = null;
  private eventGain: GainNode | null = null;
  private ambientNodes: AudioNode[] = [];
  private _muted = false;
  private _masterVolume = 0.3;
  private _ambientEnabled = true;
  private _eventEnabled = true;

  init() {
    if (this.ctx) return;
    this.ctx = new AudioContext();
    if (this.ctx.state === "suspended") {
      this.ctx.resume();
    }
    this.masterGain = this.ctx.createGain();
    this.masterGain.gain.value = this._masterVolume;
    this.masterGain.connect(this.ctx.destination);

    this.ambientGain = this.ctx.createGain();
    this.ambientGain.gain.value = 0.4;
    this.ambientGain.connect(this.masterGain);

    this.eventGain = this.ctx.createGain();
    this.eventGain.gain.value = 0.6;
    this.eventGain.connect(this.masterGain);

    if (this._ambientEnabled) this.startAmbient();
  }

  private startAmbient() {
    if (!this.ctx || !this.ambientGain) return;
    this.stopAmbient();

    // Deep underwater hum - filtered noise
    const bufferSize = this.ctx.sampleRate * 2;
    const noiseBuffer = this.ctx.createBuffer(1, bufferSize, this.ctx.sampleRate);
    const data = noiseBuffer.getChannelData(0);
    for (let i = 0; i < bufferSize; i++) {
      data[i] = Math.random() * 2 - 1;
    }

    const noise = this.ctx.createBufferSource();
    noise.buffer = noiseBuffer;
    noise.loop = true;

    const lowpass = this.ctx.createBiquadFilter();
    lowpass.type = "lowpass";
    lowpass.frequency.value = 200;
    lowpass.Q.value = 1;

    const noiseGain = this.ctx.createGain();
    noiseGain.gain.value = 0.15;

    noise.connect(lowpass);
    lowpass.connect(noiseGain);
    noiseGain.connect(this.ambientGain);
    noise.start();
    this.ambientNodes.push(noise, lowpass, noiseGain);

    // Slow LFO modulated tone for depth feel
    const osc = this.ctx.createOscillator();
    osc.type = "sine";
    osc.frequency.value = 60;

    const lfo = this.ctx.createOscillator();
    lfo.type = "sine";
    lfo.frequency.value = 0.1;
    const lfoGain = this.ctx.createGain();
    lfoGain.gain.value = 5;
    lfo.connect(lfoGain);
    lfoGain.connect(osc.frequency);

    const oscGain = this.ctx.createGain();
    oscGain.gain.value = 0.06;

    osc.connect(oscGain);
    oscGain.connect(this.ambientGain);
    osc.start();
    lfo.start();
    this.ambientNodes.push(osc, lfo, lfoGain, oscGain);
  }

  private stopAmbient() {
    for (const node of this.ambientNodes) {
      if (node instanceof AudioScheduledSourceNode) {
        try { node.stop(); } catch { /* already stopped */ }
      }
      node.disconnect();
    }
    this.ambientNodes = [];
  }

  // Bubble pop sound
  playBubble() {
    if (!this.ctx || !this.eventGain || !this._eventEnabled) return;
    const t = this.ctx.currentTime;
    const osc = this.ctx.createOscillator();
    osc.type = "sine";
    osc.frequency.setValueAtTime(800 + Math.random() * 400, t);
    osc.frequency.exponentialRampToValueAtTime(200, t + 0.1);

    const gain = this.ctx.createGain();
    gain.gain.setValueAtTime(0.08, t);
    gain.gain.exponentialRampToValueAtTime(0.001, t + 0.15);

    osc.connect(gain);
    gain.connect(this.eventGain);
    osc.start(t);
    osc.stop(t + 0.15);
    osc.onended = () => { osc.disconnect(); gain.disconnect(); };
  }

  // Birth chime
  playBirth() {
    if (!this.ctx || !this.eventGain || !this._eventEnabled) return;
    const t = this.ctx.currentTime;
    [523, 659, 784].forEach((freq, i) => {
      const osc = this.ctx!.createOscillator();
      osc.type = "sine";
      osc.frequency.value = freq;
      const gain = this.ctx!.createGain();
      gain.gain.setValueAtTime(0, t + i * 0.08);
      gain.gain.linearRampToValueAtTime(0.1, t + i * 0.08 + 0.02);
      gain.gain.exponentialRampToValueAtTime(0.001, t + i * 0.08 + 0.3);
      osc.connect(gain);
      gain.connect(this.eventGain!);
      osc.start(t + i * 0.08);
      osc.stop(t + i * 0.08 + 0.3);
      osc.onended = () => { osc.disconnect(); gain.disconnect(); };
    });
  }

  // Death tone
  playDeath() {
    if (!this.ctx || !this.eventGain || !this._eventEnabled) return;
    const t = this.ctx.currentTime;
    const osc = this.ctx.createOscillator();
    osc.type = "triangle";
    osc.frequency.setValueAtTime(300, t);
    osc.frequency.linearRampToValueAtTime(150, t + 0.5);
    const gain = this.ctx.createGain();
    gain.gain.setValueAtTime(0.1, t);
    gain.gain.linearRampToValueAtTime(0, t + 0.6);
    osc.connect(gain);
    gain.connect(this.eventGain);
    osc.start(t);
    osc.stop(t + 0.6);
    osc.onended = () => { osc.disconnect(); gain.disconnect(); };
  }

  // New species fanfare
  playNewSpecies() {
    if (!this.ctx || !this.eventGain || !this._eventEnabled) return;
    const t = this.ctx.currentTime;
    [440, 554, 659, 880].forEach((freq, i) => {
      const osc = this.ctx!.createOscillator();
      osc.type = "sine";
      osc.frequency.value = freq;
      const gain = this.ctx!.createGain();
      gain.gain.setValueAtTime(0, t + i * 0.1);
      gain.gain.linearRampToValueAtTime(0.12, t + i * 0.1 + 0.03);
      gain.gain.exponentialRampToValueAtTime(0.001, t + i * 0.1 + 0.4);
      osc.connect(gain);
      gain.connect(this.eventGain!);
      osc.start(t + i * 0.1);
      osc.stop(t + i * 0.1 + 0.4);
      osc.onended = () => { osc.disconnect(); gain.disconnect(); };
    });
  }

  // Extinction low rumble
  playExtinction() {
    if (!this.ctx || !this.eventGain || !this._eventEnabled) return;
    const t = this.ctx.currentTime;
    const osc = this.ctx.createOscillator();
    osc.type = "sawtooth";
    osc.frequency.setValueAtTime(80, t);
    osc.frequency.linearRampToValueAtTime(40, t + 0.8);
    const filter = this.ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.value = 200;
    const gain = this.ctx.createGain();
    gain.gain.setValueAtTime(0.15, t);
    gain.gain.linearRampToValueAtTime(0, t + 1.0);
    osc.connect(filter);
    filter.connect(gain);
    gain.connect(this.eventGain);
    osc.start(t);
    osc.stop(t + 1.0);
    osc.onended = () => { osc.disconnect(); filter.disconnect(); gain.disconnect(); };
  }

  // Feed splash
  playFeed() {
    if (!this.ctx || !this.eventGain || !this._eventEnabled) return;
    const t = this.ctx.currentTime;
    // Short noise burst filtered for splash effect
    const bufferSize = Math.round(this.ctx.sampleRate * 0.1);
    const buffer = this.ctx.createBuffer(1, bufferSize, this.ctx.sampleRate);
    const d = buffer.getChannelData(0);
    for (let i = 0; i < bufferSize; i++) {
      d[i] = (Math.random() * 2 - 1) * (1 - i / bufferSize);
    }
    const src = this.ctx.createBufferSource();
    src.buffer = buffer;
    const bp = this.ctx.createBiquadFilter();
    bp.type = "bandpass";
    bp.frequency.value = 2000;
    bp.Q.value = 0.5;
    const gain = this.ctx.createGain();
    gain.gain.setValueAtTime(0.12, t);
    gain.gain.exponentialRampToValueAtTime(0.001, t + 0.12);
    src.connect(bp);
    bp.connect(gain);
    gain.connect(this.eventGain);
    src.start(t);
    src.onended = () => { src.disconnect(); bp.disconnect(); gain.disconnect(); };
  }

  playTap() {
    if (!this.ctx || !this.eventGain || !this._eventEnabled) return;
    const t = this.ctx.currentTime;
    // Short percussive knock sound — noise burst through low-pass filter
    const bufferSize = Math.round(this.ctx.sampleRate * 0.08);
    const buffer = this.ctx.createBuffer(1, bufferSize, this.ctx.sampleRate);
    const d = buffer.getChannelData(0);
    for (let i = 0; i < bufferSize; i++) {
      d[i] = (Math.random() * 2 - 1) * Math.pow(1 - i / bufferSize, 3);
    }
    const src = this.ctx.createBufferSource();
    src.buffer = buffer;
    const lp = this.ctx.createBiquadFilter();
    lp.type = "lowpass";
    lp.frequency.value = 800;
    lp.Q.value = 1.2;
    const gain = this.ctx.createGain();
    gain.gain.setValueAtTime(0.18, t);
    gain.gain.exponentialRampToValueAtTime(0.001, t + 0.1);
    src.connect(lp);
    lp.connect(gain);
    gain.connect(this.eventGain);
    src.start(t);
    src.onended = () => { src.disconnect(); lp.disconnect(); gain.disconnect(); };
  }

  set muted(v: boolean) {
    this._muted = v;
    if (this.masterGain) {
      this.masterGain.gain.value = v ? 0 : this._masterVolume;
    }
  }

  get muted() {
    return this._muted;
  }

  set masterVolume(v: number) {
    const normalizedVolume = normalizeVolume(v);
    this._masterVolume = normalizedVolume;
    if (this.masterGain && !this._muted) {
      this.masterGain.gain.value = normalizedVolume;
    }
  }

  get masterVolume() {
    return this._masterVolume;
  }

  set ambientEnabled(v: boolean) {
    this._ambientEnabled = v;
    if (v && this.ctx) this.startAmbient();
    else this.stopAmbient();
  }

  set eventEnabled(v: boolean) {
    this._eventEnabled = v;
  }

  destroy() {
    this.stopAmbient();
    this.ctx?.close();
    this.ctx = null;
    this.masterGain = null;
    this.ambientGain = null;
    this.eventGain = null;
  }
}
