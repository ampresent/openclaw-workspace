class AudioEngine {
  constructor() {
    this.ctx = null;
    this.bugA = null;
    this.bugB = null;
    this.masterGain = null;
    this.bpm = 120;
    this.beatInterval = 60 / this.bpm;
    this.nextBeatTime = 0;
    this.beatCount = 0;
  }

  init() {
    this.ctx = new (window.AudioContext || window.webkitAudioContext)();
    this.masterGain = this.ctx.createGain();
    this.masterGain.gain.value = 0.3;
    this.masterGain.connect(this.ctx.destination);
    this.bugA = this._createVoice('sawtooth', 110, 300);
    this.bugB = this._createVoice('square', 220, 4000);
  }

  _createVoice(waveform, freq, filterFreq) {
    const osc = this.ctx.createOscillator();
    osc.type = waveform;
    osc.frequency.value = freq;
    const filter = this.ctx.createBiquadFilter();
    filter.type = 'lowpass';
    filter.frequency.value = filterFreq;
    filter.Q.value = 2;
    const gain = this.ctx.createGain();
    gain.gain.value = 0.4;
    const analyser = this.ctx.createAnalyser();
    analyser.fftSize = 256;
    osc.connect(filter);
    filter.connect(gain);
    gain.connect(analyser);
    analyser.connect(this.masterGain);
    osc.start();
    return { osc, filter, gain, analyser };
  }

  setPlayerFilter(freq) {
    if (!this.bugA) return;
    this.bugA.filter.frequency.setTargetAtTime(freq, this.ctx.currentTime, 0.05);
  }

  setAIFilter(freq) {
    if (!this.bugB) return;
    this.bugB.filter.frequency.setTargetAtTime(freq, this.ctx.currentTime, 0.05);
  }

  getFrequencyData(voice) {
    const data = new Uint8Array(voice.analyser.frequencyBinCount);
    voice.analyser.getByteFrequencyData(data);
    return data;
  }

  getCurrentChord(time) {
    const chords = [
      [261.63, 329.63, 392.00],
      [220.00, 277.18, 329.63],
      [246.94, 311.13, 369.99],
      [196.00, 246.94, 293.66],
    ];
    const idx = Math.floor(time / (this.beatInterval * 4)) % chords.length;
    return chords[idx];
  }

  isOnBeat(time) {
    const phase = (time % this.beatInterval) / this.beatInterval;
    return phase < 0.05 || phase > 0.95;
  }

  isHarmonic(freqA, freqB) {
    const ratio = Math.max(freqA, freqB) / Math.min(freqA, freqB);
    const intervals = [1, 1.5, 1.25, 1.333, 2, 2.5, 3];
    return intervals.some(i => Math.abs(ratio - i) < 0.05);
  }

  computeHarmony(dataA, dataB) {
    let harmonic = 0, conflict = 0;
    const len = Math.min(dataA.length, dataB.length);
    for (let i = 0; i < len; i++) {
      const a = dataA[i] / 255;
      const b = dataB[i] / 255;
      if (a > 0.1 && b > 0.1) {
        if (a > 0.5 && b > 0.5) {
          conflict += (a * b);
        } else {
          harmonic += Math.min(a, b);
        }
      }
    }
    const total = harmonic + conflict || 1;
    return { harmonic: harmonic / total, conflict: conflict / total };
  }

  updateMelody(time) {
    const chord = this.getCurrentChord(time);
    if (this.isOnBeat(time)) {
      const noteA = chord[Math.floor(Math.random() * chord.length)];
      this.bugA.osc.frequency.setTargetAtTime(noteA, this.ctx.currentTime, 0.1);
      const noteB = chord[Math.floor(Math.random() * chord.length)] * 2;
      this.bugB.osc.frequency.setTargetAtTime(noteB, this.ctx.currentTime, 0.1);
    }
  }
}
