#!/usr/bin/env python3
"""
TidalCycles Output Synthesizer & Similarity Comparator (v2).
Usage: python3 compare.py <original.wav> <tidal_params.json> [output_dir]

v2: Signal chain architecture with chords, saturation, dub delay,
    resonant filters, reverb, and LFO modulation.

Metrics (10 total, frequency + time + harmonic):
  Frequency: Band Cosine, Centroid, Band Correlation, MFCC
  Time:      RMS Correlation, Onset Pattern, Temporal Envelope, Beat Structure
  New:       Harmonic Similarity, Delay Tail Match
"""
import numpy as np
import librosa
import soundfile as sf
from scipy.signal import butter, filtfilt, iirfilter, sosfilt
import json, os, sys

SR = 44100

# ============================================================
# Synthesis building blocks
# ============================================================

def note_to_freq(note):
    try:
        return librosa.note_to_hz(note)
    except:
        return 130.81

def synth_sine(freq, dur, sr=SR):
    t = np.linspace(0, dur, int(sr * dur), False)
    return np.sin(2 * np.pi * freq * t)

def synth_supersaw(freq, dur, detune=1.004, sr=SR):
    t = np.linspace(0, dur, int(sr * dur), False)
    sig = np.zeros_like(t)
    for df in [1/detune, 1, detune]:
        sig += np.sin(2 * np.pi * freq * df * t) * 0.33
        sig += np.sin(2 * np.pi * freq * 2 * df * t) * 0.12
        sig += np.sin(2 * np.pi * freq * 3 * df * t) * 0.06
    return sig

def synth_superfm(freq, dur, mod_ratio=2, mod_index=2.0, sr=SR):
    t = np.linspace(0, dur, int(sr * dur), False)
    mod = np.sin(2 * np.pi * freq * mod_ratio * t) * mod_index
    return np.sin(2 * np.pi * freq * t + mod) * 0.5

def synth_noise(dur, sr=SR):
    return np.random.randn(int(sr * dur))

def synth_chord(notes, dur, synth_type='supersaw', detune=1.004, sr=SR):
    """Synthesize a chord (multiple notes mixed together)."""
    sig = np.zeros(int(sr * dur))
    for note in notes:
        freq = note_to_freq(note)
        if synth_type == 'sine':
            sig += synth_sine(freq, dur, sr)
        elif synth_type == 'supersaw':
            sig += synth_supersaw(freq, dur, detune, sr)
        elif synth_type == 'superfm':
            sig += synth_superfm(freq, dur, sr=sr)
        else:
            sig += synth_sine(freq, dur, sr)
    # Normalize per note count
    sig /= max(len(notes), 1)
    return sig

# ============================================================
# Effects / processors (signal chain modules)
# ============================================================

def apply_lpf(sig, cutoff, order=2, sr=SR):
    nyq = sr / 2
    if cutoff >= nyq: return sig
    b, a = butter(order, min(cutoff, nyq - 1) / nyq, 'low')
    return filtfilt(b, a, sig)

def apply_hpf(sig, cutoff, order=2, sr=SR):
    nyq = sr / 2
    if cutoff >= nyq: return sig
    b, a = butter(order, min(cutoff, nyq - 1) / nyq, 'high')
    return filtfilt(b, a, sig)

def apply_resonant_lpf(sig, cutoff, resonance=0.5, sr=SR):
    """Resonant LPF using high-order IIR with feedback simulation."""
    nyq = sr / 2
    wc = min(cutoff / nyq, 0.99)
    # Higher order for more resonance character
    q = 1.0 / (1.0 - resonance + 0.01)  # Q factor from resonance 0-0.99
    order = min(8, max(2, int(q * 2)))
    if order % 2 != 0:
        order += 1
    try:
        sos = iirfilter(order, wc, btype='low', ftype='butter', output='sos')
        out = sosfilt(sos, sig)
        # Mix with dry for resonance peak
        if resonance > 0.3:
            # Boost near cutoff frequency (simulated resonance)
            peak_boost = 1.0 + resonance * 2.0
            # Apply boost as a simple shelving approximation
            hi_part = sig - apply_lpf(sig, cutoff * 0.5, sr=sr)
            out += hi_part * (resonance - 0.3) * 0.5
        return out
    except:
        return apply_lpf(sig, cutoff, sr=sr)

def apply_lfo_modulated_filter(sig, base_cutoff, lfo_rate, lfo_depth, resonance=0.0, sr=SR):
    """LFO-modulated filter cutoff — classic dub techno sweep."""
    n = len(sig)
    t = np.arange(n) / sr
    # Cutoff oscillates: base_cutoff * (1 + depth * sin(2π * rate * t))
    cutoff_mod = base_cutoff * (1.0 + lfo_depth * np.sin(2 * np.pi * lfo_rate * t))
    cutoff_mod = np.clip(cutoff_mod, 20, sr / 2 - 1)

    # Process in chunks for time-varying filter
    chunk_size = 2048
    out = np.zeros_like(sig)
    for i in range(0, n, chunk_size):
        end = min(i + chunk_size, n)
        local_cutoff = float(np.mean(cutoff_mod[i:end]))
        if resonance > 0:
            out[i:end] = apply_resonant_lpf(sig[i:end], local_cutoff, resonance, sr)
        else:
            out[i:end] = apply_lpf(sig[i:end], local_cutoff, sr=sr)
    return out

def apply_envelope(sig, attack=0.01, release=0.1, sr=SR):
    out = sig.copy()
    n = len(out)
    a = int(attack * sr)
    r = int(release * sr)
    if 0 < a < n: out[:a] *= np.linspace(0, 1, a)
    if 0 < r < n: out[-r:] *= np.linspace(1, 0, r)
    return out

def saturate_tape(sig, drive=2.0):
    """Tape saturation — asymmetric soft clip generating even harmonics."""
    driven = sig * drive
    # Asymmetric: positive and negative clip differently
    out = np.where(driven >= 0,
                   1.0 - np.exp(-driven),  # positive: exponential soft clip
                   -1.0 + np.exp(driven))  # negative: steeper
    out = np.tanh(driven) * 0.7 + out * 0.3  # blend
    # Normalize to prevent level creep
    peak = np.max(np.abs(out)) + 1e-8
    out *= min(1.0, 0.9 / peak)
    return out

def saturate_tube(sig, drive=1.5):
    """Tube saturation — odd harmonics via sign-based soft clip."""
    driven = sig * drive
    out = np.sign(driven) * (1.0 - np.exp(-np.abs(driven)))
    return out * 0.9

def saturate_wavefolder(sig, drive=1.0):
    """Wavefolder — extreme harmonics, good for metallic textures."""
    driven = sig * drive
    # Fold: reflect values outside [-1, 1]
    out = driven.copy()
    over = np.abs(out) > 1.0
    out[over] = 2.0 - np.abs(out[over])
    out = np.clip(out, -1, 1)
    return out

def apply_dub_delay(sig, delay_time, feedback=0.6, lpf_cutoff=3000, mix=0.4, sr=SR):
    """Dub-style delay: each tap goes through LPF (gets darker)."""
    n = len(sig)
    d = int(delay_time * sr)
    if d <= 0 or d >= n:
        return sig

    out = sig.copy()
    # How many taps until energy is negligible
    max_taps = int(np.log(0.001) / np.log(max(feedback, 0.01))) + 1
    max_taps = min(max_taps, 64)

    current = sig.copy()
    current_cut = lpf_cutoff
    for tap in range(max_taps):
        offset = d * (tap + 1)
        if offset >= n:
            break
        # Apply LPF to the delayed signal (gets darker each tap)
        current = apply_lpf(current, current_cut, sr=sr)
        gain = feedback ** (tap + 1)
        length = min(len(current), n - offset)
        out[offset:offset + length] += current[:length] * gain * mix
        # Each repeat gets darker
        current_cut *= 0.85
        current_cut = max(current_cut, 100)

    return out

def apply_reverb(sig, size=0.5, damp=0.5, mix=0.3, sr=SR):
    """Simple algorithmic reverb using nested allpass + comb filters."""
    n = len(sig)
    if n == 0:
        return sig

    # Comb filter delays (in samples) — Schroeder reverb
    comb_delays = [int(d * sr) for d in [0.0297, 0.0371, 0.0411, 0.0437]]
    comb_gains = [0.7 * size + 0.1 for _ in comb_delays]

    # Allpass delays
    ap_delays = [int(d * sr) for d in [0.005, 0.0017]]
    ap_gains = [0.7, 0.7]

    # Parallel comb filters
    comb_out = np.zeros(n)
    for delay, gain in zip(comb_delays, comb_gains):
        buf = np.zeros(n)
        for i in range(delay, n):
            buf[i] = sig[i] + gain * buf[i - delay]
            # Damping: LPF on feedback
            if damp > 0 and i > 0:
                buf[i] = buf[i] * (1 - damp * 0.3) + buf[i - 1] * damp * 0.3
        comb_out += buf
    comb_out /= len(comb_delays)

    # Series allpass filters
    rev = comb_out.copy()
    for delay, gain in zip(ap_delays, ap_gains):
        buf = np.zeros(n)
        for i in range(delay, n):
            buf[i] = -gain * rev[i] + rev[i - delay] + gain * buf[i - delay]
        rev = buf

    # Mix
    out = sig * (1 - mix) + rev * mix
    return out

def apply_place(buf, sig, start_sec, gain=1.0, sr=SR):
    s = int(start_sec * sr)
    e = min(s + len(sig), len(buf))
    if s < len(buf) and e > s:
        buf[s:e] += sig[:e - s] * gain


# ============================================================
# Signal chain synthesis (v2 — replaces synthesize_tidal)
# ============================================================

def synthesize_signal_chain(chain_def, duration, sr=SR):
    """
    Signal chain architecture. Each chain is a dict:
    {
      "source": {"notes": ["g2","a#2","d3"], "synth": "supersaw", "detune": 1.005},
      "filter": {"lpf": 400, "resonance": 0.7, "lfo_rate": 0.15, "lfo_depth": 0.6},
      "saturate": {"type": "tape", "drive": 2.5},
      "delay": {"time": 0.366, "feedback": 0.7, "lpf": 2000, "mix": 0.4},
      "reverb": {"size": 0.6, "damp": 0.5, "mix": 0.25},
      "pattern": {"speed": 1, "interval": 1.0, "degrade": 0.1,
                  "sustain": 0.15, "attack": 0.003, "release": 0.08},
      "gain": 0.8
    }
    """
    bpm = chain_def.get('_bpm', 120)
    beat_dur = 60.0 / bpm
    buf = np.zeros(int(sr * duration))

    chains = chain_def.get('chains', [])
    for chain in chains:
        src = chain.get('source', {})
        filt = chain.get('filter', {})
        sat = chain.get('saturate', {})
        dly = chain.get('delay', {})
        rev = chain.get('reverb', {})
        pat = chain.get('pattern', {})
        chain_gain = chain.get('gain', 0.5)

        notes = src.get('notes', ['c3'])
        synth_type = src.get('synth', 'sine')
        detune = src.get('detune', 1.004)
        sustain = pat.get('sustain', 0.2)
        attack = pat.get('attack', 0.01)
        release = pat.get('release', 0.05)
        speed = pat.get('speed', 1)
        interval = pat.get('interval', 1.0)
        degrade = pat.get('degrade', 0)

        step = beat_dur * interval / speed
        for t in np.arange(0, duration - sustain, step):
            if degrade > 0 and np.random.random() < degrade:
                continue

            # 1. Source: chord or single note
            if len(notes) > 1:
                sig = synth_chord(notes, sustain, synth_type, detune, sr)
            else:
                freq = note_to_freq(notes[0])
                if synth_type == 'sine':
                    sig = synth_sine(freq, sustain, sr)
                elif synth_type == 'supersaw':
                    sig = synth_supersaw(freq, sustain, detune, sr)
                elif synth_type == 'superfm':
                    sig = synth_superfm(freq, sustain, sr=sr)
                elif synth_type == 'noise':
                    sig = synth_noise(sustain, sr)
                else:
                    sig = synth_sine(freq, sustain, sr)

            # 2. Amplitude envelope
            sig = apply_envelope(sig, attack, release, sr)

            # 3. Filter (resonant + LFO modulated)
            lpf_cut = filt.get('lpf', sr / 2 - 1)
            hpf_cut = filt.get('hpf', 0)
            resonance = filt.get('resonance', 0)
            lfo_rate = filt.get('lfo_rate', 0)
            lfo_depth = filt.get('lfo_depth', 0)

            if lfo_rate > 0 and lfo_depth > 0:
                sig = apply_lfo_modulated_filter(sig, lpf_cut, lfo_rate, lfo_depth, resonance, sr)
            elif resonance > 0:
                sig = apply_resonant_lpf(sig, lpf_cut, resonance, sr)
            elif lpf_cut < sr / 2 - 1:
                sig = apply_lpf(sig, lpf_cut, sr=sr)
            if hpf_cut > 0:
                sig = apply_hpf(sig, hpf_cut, sr=sr)

            # 4. Saturation
            sat_type = sat.get('type', '')
            sat_drive = sat.get('drive', 0)
            if sat_drive > 0 and sat_type:
                if sat_type == 'tape':
                    sig = saturate_tape(sig, sat_drive)
                elif sat_type == 'tube':
                    sig = saturate_tube(sig, sat_drive)
                elif sat_type == 'wavefolder':
                    sig = saturate_wavefolder(sig, sat_drive)
                else:
                    sig = saturate_tape(sig, sat_drive)

            # 5. Delay (dub style)
            dly_time = dly.get('time', 0)
            dly_fb = dly.get('feedback', 0.3)
            dly_lpf = dly.get('lpf', 4000)
            dly_mix = dly.get('mix', 0)
            if dly_mix > 0 and dly_time > 0:
                sig = apply_dub_delay(sig, dly_time, dly_fb, dly_lpf, dly_mix, sr)

            # 6. Reverb
            rev_size = rev.get('size', 0)
            rev_damp = rev.get('damp', 0.5)
            rev_mix = rev.get('mix', 0)
            if rev_mix > 0:
                sig = apply_reverb(sig, rev_size, rev_damp, rev_mix, sr)

            # Place in buffer
            apply_place(buf, sig, t, chain_gain, sr)

    # Master processing
    if np.max(np.abs(buf)) > 0:
        buf = apply_lpf(buf, 12000, sr=sr)
        buf = np.tanh(buf * 0.85) * 0.75
        buf /= (np.max(np.abs(buf)) + 1e-8) * 0.85
    return buf


def synthesize_legacy_orbits(params, duration, sr=SR):
    """Backward-compatible orbit synthesis (v1 format)."""
    bpm = params.get('_bpm', 120)
    beat_dur = 60.0 / bpm
    buf = np.zeros(int(sr * duration))
    orbits = params.get('orbits', [])

    for orbit in orbits:
        notes = orbit.get('notes', None) or [orbit.get('note', 'c3')]
        synth_type = orbit.get('synth', 'sine')
        gain = orbit.get('gain', 0.5)
        lpf_cut = orbit.get('lpf', sr / 2 - 1)
        hpf_cut = orbit.get('hpf', 0)
        sustain = orbit.get('sustain', 0.2)
        attack = orbit.get('attack', 0.01)
        release = orbit.get('release', 0.05)
        speed = orbit.get('speed', 1)
        degrade = orbit.get('degrade', 0)
        interval = orbit.get('interval', 1.0)
        detune = orbit.get('detune', 1.004)
        delay_mix = orbit.get('delay', 0)
        delay_time = orbit.get('delaytime', 0.25)
        delay_fb = orbit.get('delayfeedback', 0.3)
        delay_lpf = orbit.get('delaylpf', 4000)
        saturation = orbit.get('saturation', 0)
        sat_type = orbit.get('sattype', 'tape')

        step = beat_dur * interval / speed
        for t in np.arange(0, duration - sustain, step):
            if degrade > 0 and np.random.random() < degrade:
                continue

            if len(notes) > 1:
                sig = synth_chord(notes, sustain, synth_type, detune, sr)
            else:
                freq = note_to_freq(notes[0])
                if synth_type == 'sine':
                    sig = synth_sine(freq, sustain, sr)
                elif synth_type == 'supersaw':
                    sig = synth_supersaw(freq, sustain, detune, sr)
                elif synth_type == 'superfm':
                    mod_ratio = orbit.get('mod_ratio', 2)
                    mod_index = orbit.get('mod_index', 2.0)
                    sig = synth_superfm(freq, sustain, mod_ratio, mod_index, sr)
                elif synth_type == 'noise':
                    sig = synth_noise(sustain, sr)
                else:
                    sig = synth_sine(freq, sustain, sr)

            sig = apply_envelope(sig, attack, release, sr)
            resonance = orbit.get('resonance', 0)
            if resonance > 0:
                sig = apply_resonant_lpf(sig, lpf_cut, resonance, sr)
            elif lpf_cut < sr / 2 - 1:
                sig = apply_lpf(sig, lpf_cut, sr=sr)
            if hpf_cut > 0:
                sig = apply_hpf(sig, hpf_cut, sr=sr)
            if saturation > 0:
                if sat_type == 'tape':
                    sig = saturate_tape(sig, saturation)
                elif sat_type == 'tube':
                    sig = saturate_tube(sig, saturation)
                else:
                    sig = saturate_tape(sig, saturation)
            if delay_mix > 0:
                sig = apply_dub_delay(sig, delay_time, delay_fb, delay_lpf, delay_mix, sr)

            apply_place(buf, sig, t, gain, sr)

    if np.max(np.abs(buf)) > 0:
        buf = apply_lpf(buf, 12000, sr=sr)
        buf = np.tanh(buf * 0.85) * 0.75
        buf /= (np.max(np.abs(buf)) + 1e-8) * 0.85
    return buf


def synthesize_tidal(params, duration, sr=SR):
    """Auto-detect format: signal chain or legacy orbits."""
    if 'chains' in params:
        return synthesize_signal_chain(params, duration, sr)
    else:
        return synthesize_legacy_orbits(params, duration, sr)


# ============================================================
# Similarity metrics
# ============================================================

def compute_onset_pattern_similarity(y_orig, y_synth, sr=SR):
    env_o = librosa.onset.onset_strength(y=y_orig, sr=sr)
    env_s = librosa.onset.onset_strength(y=y_synth, sr=sr)
    onsets_o = librosa.onset.onset_detect(y=y_orig, sr=sr, backtrack=False)
    onsets_s = librosa.onset.onset_detect(y=y_synth, sr=sr, backtrack=False)
    times_o = librosa.frames_to_time(onsets_o, sr=sr)
    times_s = librosa.frames_to_time(onsets_s, sr=sr)
    intervals_o = np.diff(times_o) if len(times_o) > 1 else np.array([0])
    intervals_s = np.diff(times_s) if len(times_s) > 1 else np.array([0])

    bins = np.linspace(0, 2.0, 41)
    hist_o, _ = np.histogram(intervals_o, bins=bins, density=True)
    hist_s, _ = np.histogram(intervals_s, bins=bins, density=True)
    hist_corr = float(np.corrcoef(hist_o, hist_s)[0, 1]) if np.std(hist_o) > 1e-10 and np.std(hist_s) > 1e-10 else 0.0

    dur = max(len(y_orig), len(y_synth)) / sr
    win = 0.5
    nw = max(1, int(dur / win) + 1)
    def density(times, n):
        d = np.zeros(n)
        for t in times:
            d[min(int(t / win), n - 1)] += 1
        return d
    den_o, den_s = density(times_o, nw), density(times_s, nw)
    ml = min(len(den_o), len(den_s))
    den_corr = float(np.corrcoef(den_o[:ml], den_s[:ml])[0, 1]) if ml > 1 and np.std(den_o[:ml]) > 1e-10 and np.std(den_s[:ml]) > 1e-10 else 0.0

    def compress_env(env, n_bins=100):
        if len(env) == 0: return np.zeros(n_bins)
        chunk = max(1, len(env) // n_bins)
        return np.array([np.mean(env[i*chunk:(i+1)*chunk]) for i in range(n_bins)])[:n_bins]
    comp_o, comp_s = compress_env(env_o), compress_env(env_s)
    env_corr = float(np.corrcoef(comp_o, comp_s)[0, 1]) if np.std(comp_o) > 1e-10 and np.std(comp_s) > 1e-10 else 0.0

    similarity = 0.4 * max(0, hist_corr) + 0.3 * max(0, den_corr) + 0.3 * max(0, env_corr)
    return similarity, {'histogram_corr': round(hist_corr, 4), 'density_corr': round(den_corr, 4), 'onset_env_corr': round(env_corr, 4)}


def compute_temporal_centroid_similarity(y_orig, y_synth, sr=SR):
    n_bins = 100
    def temporal_profile(y, n):
        rms = librosa.feature.rms(y=y, hop_length=512)[0]
        if len(rms) == 0: return np.zeros(n)
        chunk = max(1, len(rms) // n)
        profile = np.array([np.mean(rms[i*chunk:(i+1)*chunk]) for i in range(n)])[:n]
        if np.sum(profile) > 0: profile /= np.sum(profile)
        return profile
    prof_o, prof_s = temporal_profile(y_orig, n_bins), temporal_profile(y_synth, n_bins)
    corr = float(np.corrcoef(prof_o, prof_s)[0, 1]) if np.std(prof_o) > 1e-10 and np.std(prof_s) > 1e-10 else 0.0
    tc_o, tc_s = float(np.sum(prof_o * np.arange(n_bins))), float(np.sum(prof_s * np.arange(n_bins)))
    tc_sim = 1.0 - abs(tc_o - tc_s) / n_bins
    spread_o = float(np.std(np.arange(n_bins) * prof_o / (np.sum(prof_o) + 1e-10)))
    spread_s = float(np.std(np.arange(n_bins) * prof_s / (np.sum(prof_s) + 1e-10)))
    spread_sim = 1.0 - abs(spread_o - spread_s) / (max(spread_o, spread_s) + 1e-10)
    similarity = 0.4 * max(0, corr) + 0.3 * max(0, tc_sim) + 0.3 * max(0, spread_sim)
    return similarity, {'profile_corr': round(corr, 4), 'temporal_centroid_sim': round(tc_sim, 4), 'spread_sim': round(spread_sim, 4)}


def compute_beat_energy_similarity(y_orig, y_synth, sr=SR):
    _, beats_o = librosa.beat.beat_track(y=y_orig, sr=sr)
    _, beats_s = librosa.beat.beat_track(y=y_synth, sr=sr)
    beats_o, beats_s = np.atleast_1d(beats_o), np.atleast_1d(beats_s)
    rms_o = librosa.feature.rms(y=y_orig, hop_length=512)[0]
    rms_s = librosa.feature.rms(y=y_synth, hop_length=512)[0]

    def beat_prof(beats, arr, max_beats=64):
        if len(beats) < 3: return np.array([])
        prof = []
        for i in range(min(len(beats) - 1, max_beats)):
            s, e = beats[i], beats[i+1]
            if e > s: prof.append(float(np.mean(arr[s:e])))
        return np.array(prof)

    brms_o, brms_s = beat_prof(beats_o, rms_o), beat_prof(beats_s, rms_s)
    ml = min(len(brms_o), len(brms_s))
    beat_corr = float(np.corrcoef(brms_o[:ml], brms_s[:ml])[0, 1]) if ml > 2 and np.std(brms_o[:ml]) > 1e-10 and np.std(brms_s[:ml]) > 1e-10 else 0.0

    cent_o = librosa.feature.spectral_centroid(y=y_orig, sr=sr)[0]
    cent_s = librosa.feature.spectral_centroid(y=y_synth, sr=sr)[0]
    bc_o, bc_s = beat_prof(beats_o, cent_o), beat_prof(beats_s, cent_s)
    ml_c = min(len(bc_o), len(bc_s))
    cent_corr = float(np.corrcoef(bc_o[:ml_c], bc_s[:ml_c])[0, 1]) if ml_c > 2 and np.std(bc_o[:ml_c]) > 1e-10 and np.std(bc_s[:ml_c]) > 1e-10 else 0.0

    var_o = float(np.std(brms_o)) if len(brms_o) > 0 else 0
    var_s = float(np.std(brms_s)) if len(brms_s) > 0 else 0
    var_sim = 1.0 - abs(var_o - var_s) / (max(var_o, var_s) + 1e-10)

    similarity = 0.4 * max(0, beat_corr) + 0.35 * max(0, cent_corr) + 0.25 * max(0, var_sim)
    return similarity, {'beat_rms_corr': round(beat_corr, 4), 'beat_centroid_corr': round(cent_corr, 4), 'dynamic_contrast_sim': round(var_sim, 4)}


def compute_harmonic_similarity(y_orig, y_synth, sr=SR):
    """Compare harmonic content: fundamental, harmonic series energy ratio, saturation level."""
    So = np.abs(librosa.stft(y_orig, n_fft=4096, hop_length=512))
    St = np.abs(librosa.stft(y_synth, n_fft=4096, hop_length=512))
    fr = librosa.fft_frequencies(sr=sr, n_fft=4096)
    ao, at = np.mean(So, axis=1), np.mean(St, axis=1)

    # High-frequency energy ratio (saturation indicator)
    li_h = np.searchsorted(fr, 2000)
    hi_h = np.searchsorted(fr, 10000)
    li_l = np.searchsorted(fr, 100)
    hi_l = np.searchsorted(fr, 500)

    lo_o = float(np.mean(ao[li_l:hi_l] ** 2))
    hi_o = float(np.mean(ao[li_h:hi_h] ** 2))
    lo_s = float(np.mean(at[li_l:hi_l] ** 2))
    hi_s = float(np.mean(at[li_h:hi_h] ** 2))

    sat_ratio_o = hi_o / (lo_o + 1e-10)
    sat_ratio_s = hi_s / (lo_s + 1e-10)
    sat_sim = 1.0 - abs(sat_ratio_o - sat_ratio_s) / (max(sat_ratio_o, sat_ratio_s) + 1e-10)

    # Harmonic peak alignment
    from scipy.signal import find_peaks as fp
    pk_o = fp(ao, height=np.max(ao)*0.1, distance=10)[0]
    pk_s = fp(at, height=np.max(at)*0.1, distance=10)[0]

    if len(pk_o) > 0 and len(pk_s) > 0:
        # Compare peak positions (normalized)
        f0_o = fr[pk_o[0]]
        f0_s = fr[pk_s[0]]
        f0_sim = 1.0 - abs(f0_o - f0_s) / (max(f0_o, f0_s) + 1e-10)

        # Harmonic count similarity
        hc_sim = min(len(pk_o), len(pk_s)) / max(len(pk_o), len(pk_s))
    else:
        f0_sim = 0.0
        hc_sim = 0.0

    # Full high-freq energy comparison
    li_a = np.searchsorted(fr, 2000)
    hi_spec_o = ao[li_a:]
    hi_spec_s = at[li_a:]
    ml = min(len(hi_spec_o), len(hi_spec_s))
    if np.std(hi_spec_o[:ml]) > 1e-10 and np.std(hi_spec_s[:ml]) > 1e-10:
        hi_corr = float(np.corrcoef(hi_spec_o[:ml], hi_spec_s[:ml])[0, 1])
    else:
        hi_corr = 0.0

    similarity = 0.3 * max(0, sat_sim) + 0.25 * max(0, f0_sim) + 0.2 * hc_sim + 0.25 * max(0, hi_corr)
    return similarity, {
        'saturation_sim': round(sat_sim, 4),
        'fundamental_sim': round(f0_sim, 4),
        'harmonic_count_sim': round(hc_sim, 4),
        'high_freq_corr': round(hi_corr, 4),
        'orig_sat_ratio': round(sat_ratio_o, 6),
        'synth_sat_ratio': round(sat_ratio_s, 6),
    }


def compute_delay_tail_similarity(y_orig, y_synth, sr=SR):
    """Compare post-onset decay characteristics."""
    onsets_o = librosa.onset.onset_detect(y=y_orig, sr=sr, backtrack=False)
    onsets_s = librosa.onset.onset_detect(y=y_synth, sr=sr, backtrack=False)
    rms_o = librosa.feature.rms(y=y_orig, hop_length=512)[0]
    rms_s = librosa.feature.rms(y=y_synth, hop_length=512)[0]

    def extract_decay_envelope(onsets, rms, max_onsets=15):
        decays = []
        for of in onsets[:max_onsets]:
            start = of
            end = min(of + int(1.0 * sr / 512), len(rms))
            if end - start > 10:
                seg = rms[start:end]
                peak_idx = np.argmax(seg[:min(20, len(seg))])
                tail = seg[peak_idx:]
                if len(tail) > 5:
                    # Resample to fixed length (50 bins)
                    chunk = max(1, len(tail) // 50)
                    env = np.array([np.mean(tail[i*chunk:(i+1)*chunk]) for i in range(50)])[:50]
                    if np.max(env) > 0:
                        env /= np.max(env)
                    decays.append(env)
        if len(decays) > 0:
            return np.mean(decays, axis=0)
        return np.zeros(50)

    dec_o = extract_decay_envelope(onsets_o, rms_o)
    dec_s = extract_decay_envelope(onsets_s, rms_s)

    if np.std(dec_o) > 1e-10 and np.std(dec_s) > 1e-10:
        corr = float(np.corrcoef(dec_o, dec_s)[0, 1])
    else:
        corr = 0.0

    # Decay rate comparison
    def decay_rate(env):
        x = np.arange(len(env))
        log_env = np.log(env + 1e-10)
        if np.std(x) > 0 and np.std(log_env) > 0:
            slope, _ = np.polyfit(x, log_env, 1)
            return slope
        return 0.0

    dr_o = decay_rate(dec_o)
    dr_s = decay_rate(dec_s)
    if abs(dr_o) > 1e-10:
        dr_sim = 1.0 - abs(dr_o - dr_s) / abs(dr_o)
    else:
        dr_sim = 0.0

    similarity = 0.6 * max(0, corr) + 0.4 * max(0, dr_sim)
    return similarity, {
        'decay_envelope_corr': round(corr, 4),
        'decay_rate_sim': round(dr_sim, 4),
        'orig_decay_rate': round(dr_o, 4),
        'synth_decay_rate': round(dr_s, 4),
    }


def compute_similarity(orig_path, synth_audio, sr=SR):
    y_orig, _ = librosa.load(orig_path, sr=sr)
    y_synth = synth_audio
    ml = min(len(y_orig), len(y_synth))
    y_orig, y_synth = y_orig[:ml], y_synth[:ml]

    # === Frequency domain ===
    So = np.abs(librosa.stft(y_orig, n_fft=4096, hop_length=512))
    St = np.abs(librosa.stft(y_synth, n_fft=4096, hop_length=512))
    fr = librosa.fft_frequencies(sr=sr, n_fft=4096)
    ao, at = np.mean(So, axis=1), np.mean(St, axis=1)

    bands = [
        ('sub_20_40', 20, 40), ('sub_40_60', 40, 60), ('bass_60_100', 60, 100),
        ('bass_100_200', 100, 200), ('lowmid_200_400', 200, 400),
        ('mid_400_800', 400, 800), ('mid_800_1600', 800, 1600),
        ('upper_1600_3200', 1600, 3200), ('pres_3200_6400', 3200, 6400),
        ('bril_6400_12800', 6400, 12800), ('air_12800_20000', 12800, 20000),
    ]

    ov, tv = [], []
    band_detail = {}
    for nm, lo, hi in bands:
        li = np.searchsorted(fr, lo)
        hi_i = np.searchsorted(fr, min(hi, fr[-1]))
        eo = float(np.mean(ao[li:hi_i] ** 2))
        et = float(np.mean(at[li:hi_i] ** 2))
        do, dt = 10*np.log10(eo+1e-10), 10*np.log10(et+1e-10)
        ov.append(eo); tv.append(et)
        band_detail[nm] = {'orig_db': round(do, 1), 'synth_db': round(dt, 1), 'diff': round(do-dt, 1)}

    ov, tv = np.array(ov), np.array(tv)
    cos = float(np.dot(ov, tv) / (np.linalg.norm(ov) * np.linalg.norm(tv)))
    corr = float(np.corrcoef(10*np.log10(ov+1e-10), 10*np.log10(tv+1e-10))[0, 1])
    ceno = np.sum(fr*ao) / (np.sum(ao)+1e-10)
    cent = np.sum(fr*at) / (np.sum(at)+1e-10)
    cs = 1 - abs(ceno - cent) / max(ceno, cent)

    mo = np.mean(librosa.feature.mfcc(y=y_orig, sr=sr, n_mfcc=13), axis=1)
    mt = np.mean(librosa.feature.mfcc(y=y_synth, sr=sr, n_mfcc=13), axis=1)
    ms = 1 - np.linalg.norm(mo - mt) / (np.linalg.norm(mo) + np.linalg.norm(mt))

    ro = librosa.feature.rms(y=y_orig)[0]
    rt = librosa.feature.rms(y=y_synth)[0]
    mr = min(len(ro), len(rt))
    rms_corr = max(0, float(np.corrcoef(ro[:mr], rt[:mr])[0, 1])) if mr > 1 else 0

    # === Time domain ===
    onset_sim, onset_detail = compute_onset_pattern_similarity(y_orig, y_synth, sr)
    tc_sim, tc_detail = compute_temporal_centroid_similarity(y_orig, y_synth, sr)
    beat_sim, beat_detail = compute_beat_energy_similarity(y_orig, y_synth, sr)

    # === New: Harmonic + Delay ===
    harm_sim, harm_detail = compute_harmonic_similarity(y_orig, y_synth, sr)
    delay_sim, delay_detail = compute_delay_tail_similarity(y_orig, y_synth, sr)

    # === Weighted overall score (v2) ===
    labels = ['Band Cosine', 'Centroid', 'Band Corr', 'MFCC',
              'RMS Corr', 'Onset Pattern', 'Temporal Envelope', 'Beat Structure',
              'Harmonic', 'Delay Tail']
    weights = [0.10, 0.07, 0.08, 0.07,   # freq: 32%
               0.08, 0.15, 0.10, 0.15,    # time: 48%
               0.10, 0.10]                # new: 20%
    metrics = [cos, cs, corr, ms, rms_corr, onset_sim, tc_sim, beat_sim, harm_sim, delay_sim]

    overall = sum(w * m for w, m in zip(weights, metrics))

    results = {
        'overall_similarity': round(float(overall), 4),
        'pass': bool(overall >= 0.20),
        'metrics': {l: round(float(m), 4) for l, m in zip(labels, metrics)},
        'frequency_domain': {'band_comparison': band_detail, 'centroid': {'orig': round(ceno), 'synth': round(cent)}},
        'time_domain': {'onset_pattern': onset_detail, 'temporal_centroid': tc_detail, 'beat_energy': beat_detail},
        'harmonic': harm_detail,
        'delay': delay_detail,
    }

    # Print summary
    print(f"\n  {'Band':<18} {'Orig':>8} {'Synth':>8} {'Diff':>8}")
    print(f"  {'-'*44}")
    for nm in band_detail:
        b = band_detail[nm]
        print(f"  {nm:<18} {b['orig_db']:>7.1f}  {b['synth_db']:>7.1f}  {b['diff']:>+7.1f}")

    print(f"\n{'='*65}")
    print(f"  FREQUENCY DOMAIN")
    for l, m, w in zip(labels[:4], metrics[:4], weights[:4]):
        print(f"    {l:<16} {m*100:>6.1f}%  {w*100:>4.0f}%  → {w*m*100:>5.1f}%")
    print(f"\n  TIME DOMAIN")
    for l, m, w in zip(labels[4:8], metrics[4:8], weights[4:8]):
        print(f"    {l:<16} {m*100:>6.1f}%  {w*100:>4.0f}%  → {w*m*100:>5.1f}%")
    print(f"\n  HARMONIC / DELAY")
    for l, m, w in zip(labels[8:], metrics[8:], weights[8:]):
        print(f"    {l:<16} {m*100:>6.1f}%  {w*100:>4.0f}%  → {w*m*100:>5.1f}%")
    print(f"{'='*65}")
    freq_score = sum(w*m for w, m in zip(weights[:4], metrics[:4]))
    time_score = sum(w*m for w, m in zip(weights[4:8], metrics[4:8]))
    harm_score = sum(w*m for w, m in zip(weights[8:], metrics[8:]))
    print(f"  FREQ: {freq_score*100:.1f}%  TIME: {time_score*100:.1f}%  HARMONIC: {harm_score*100:.1f}%")
    print(f"  OVERALL: {overall*100:.1f}%  (≥20% → {'✅' if overall >= 0.2 else '❌'})")

    return results


# ============================================================
# Main
# ============================================================

if __name__ == '__main__':
    if len(sys.argv) < 3:
        print("Usage: python3 compare.py <original.wav> <tidal_params.json> [output_dir]")
        print("\nv2 format (signal chain):")
        print(json.dumps({
            "_bpm": 123,
            "chains": [{
                "source": {"notes": ["g2", "a#2", "d3"], "synth": "supersaw"},
                "filter": {"lpf": 400, "resonance": 0.7, "lfo_rate": 0.15, "lfo_depth": 0.6},
                "saturate": {"type": "tape", "drive": 2.5},
                "delay": {"time": 0.366, "feedback": 0.7, "lpf": 2000, "mix": 0.4},
                "reverb": {"size": 0.6, "damp": 0.5, "mix": 0.25},
                "pattern": {"speed": 1, "interval": 1.0, "degrade": 0.1, "sustain": 0.15},
                "gain": 0.8
            }]
        }, indent=2))
        print("\nv1 format (orbits, backward compatible):")
        print(json.dumps({
            "_bpm": 120,
            "orbits": [{
                "notes": ["g2", "a#2", "d3"], "synth": "supersaw", "gain": 0.8,
                "lpf": 200, "sustain": 0.15, "attack": 0.003, "speed": 1, "interval": 1.0,
                "saturation": 2.0, "sattype": "tape",
                "delay": 0.4, "delaytime": 0.366, "delayfeedback": 0.7, "delaylpf": 2000
            }]
        }, indent=2))
        sys.exit(1)

    orig_wav = sys.argv[1]
    with open(sys.argv[2]) as f:
        params = json.load(f)
    out_dir = sys.argv[3] if len(sys.argv) > 3 else os.path.dirname(orig_wav)

    y_orig, _ = librosa.load(orig_wav, sr=SR)
    duration = len(y_orig) / SR

    print(f"Synthesizing TidalCycles output ({duration:.1f}s)...")
    synth = synthesize_tidal(params, duration)
    synth_path = os.path.join(out_dir, 'tidal_synthesis.wav')
    sf.write(synth_path, synth, SR)
    print(f"Written: {synth_path}")

    print("\nComputing similarity (frequency + time + harmonic)...")
    results = compute_similarity(orig_wav, synth, SR)

    results_path = os.path.join(out_dir, 'similarity.json')
    with open(results_path, 'w') as f:
        json.dump(results, f, indent=2)
    print(f"\nResults: {results_path}")
