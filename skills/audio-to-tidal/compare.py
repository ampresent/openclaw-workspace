#!/usr/bin/env python3
"""
TidalCycles Output Synthesizer & Similarity Comparator.
Usage: python3 compare.py <original.wav> <tidal_params.json> [output_dir]

Synthesizes expected TidalCycles output from SuperDirt parameters,
then computes real spectral similarity vs original audio.
"""
import numpy as np
import librosa
import soundfile as sf
from scipy.signal import butter, filtfilt
import json, os, sys

SR = 44100

# === SuperDirt synth emulations ===

def synth_sine(freq, dur):
    t = np.linspace(0, dur, int(SR * dur), False)
    return np.sin(2 * np.pi * freq * t)

def synth_supersaw(freq, dur, detune=1.004):
    t = np.linspace(0, dur, int(SR * dur), False)
    sig = np.zeros_like(t)
    for df in [1/detune, 1, detune]:
        sig += np.sin(2 * np.pi * freq * df * t) * 0.33
        sig += np.sin(2 * np.pi * freq * 2 * df * t) * 0.12
        sig += np.sin(2 * np.pi * freq * 3 * df * t) * 0.06
    return sig

def synth_superfm(freq, dur, mod_ratio=2, mod_index=2.0):
    t = np.linspace(0, dur, int(SR * dur), False)
    mod = np.sin(2 * np.pi * freq * mod_ratio * t) * mod_index
    return np.sin(2 * np.pi * freq * t + mod) * 0.5

def synth_noise(dur):
    return np.random.randn(int(SR * dur))

def apply_lpf(sig, cutoff, order=2):
    nyq = SR / 2
    if cutoff >= nyq: return sig
    b, a = butter(order, cutoff / nyq, 'low')
    return filtfilt(b, a, sig)

def apply_hpf(sig, cutoff, order=2):
    nyq = SR / 2
    if cutoff >= nyq: return sig
    b, a = butter(order, cutoff / nyq, 'high')
    return filtfilt(b, a, sig)

def apply_envelope(sig, attack=0.01, release=0.1):
    out = sig.copy()
    n = len(out)
    a = int(attack * SR)
    r = int(release * SR)
    if 0 < a < n: out[:a] *= np.linspace(0, 1, a)
    if 0 < r < n: out[-r:] *= np.linspace(1, 0, r)
    return out

def apply_delay(sig, delay_time=0.25, feedback=0.3, mix=0.3):
    out = sig.copy()
    d = int(delay_time * SR)
    for tap in range(3):
        offset = d * (tap + 1)
        gain = feedback ** (tap + 1)
        if offset < len(out):
            out[offset:] += sig[:len(out) - offset] * gain * mix
    return out

def place(buf, sig, start_sec, gain=1.0):
    s = int(start_sec * SR)
    e = min(s + len(sig), len(buf))
    if s < len(buf) and e > s:
        buf[s:e] += sig[:e - s] * gain


def synthesize_tidal(params, duration):
    """
    params format: list of orbit dicts:
    {
      "note": "c2",
      "synth": "supersaw",  # sine|supersaw|superfm|noise
      "gain": 0.8,
      "lpf": 200,           # optional
      "hpf": 0,             # optional
      "sustain": 0.15,      # seconds
      "attack": 0.003,
      "release": 0.1,
      "speed": 1,           # pattern speed multiplier (1=every beat, 2=half beat, 0.5=every 2 beats)
      "delay": 0,           # delay mix (0-1)
      "delaytime": 0.25,    # delay time in seconds
      "delayfeedback": 0.3,
      "degrade": 0,         # probability of dropping events (0-1)
      "interval": 1.0       # beat interval multiplier (slow 4 = 4.0)
    }
    """
    bpm = params.get('_bpm', 120) if isinstance(params, dict) else 120
    beat_dur = 60.0 / bpm
    buf = np.zeros(int(SR * duration))

    orbits = params.get('orbits', []) if isinstance(params, dict) else params

    for orbit in orbits:
        note = orbit.get('note', 'c3')
        synth_type = orbit.get('synth', 'sine')
        gain = orbit.get('gain', 0.5)
        lpf_cut = orbit.get('lpf', SR / 2 - 1)
        hpf_cut = orbit.get('hpf', 0)
        sustain = orbit.get('sustain', 0.2)
        attack = orbit.get('attack', 0.01)
        release = orbit.get('release', 0.05)
        speed = orbit.get('speed', 1)
        delay_mix = orbit.get('delay', 0)
        delay_time = orbit.get('delaytime', 0.25)
        delay_fb = orbit.get('delayfeedback', 0.3)
        degrade = orbit.get('degrade', 0)
        interval = orbit.get('interval', 1.0)

        # Note to Hz
        try:
            freq = librosa.note_to_hz(note)
        except:
            freq = 130.81  # C3 fallback

        step = beat_dur * interval / speed
        for t in np.arange(0, duration - sustain, step):
            if degrade > 0 and np.random.random() < degrade:
                continue

            if synth_type == 'sine':
                sig = synth_sine(freq, sustain)
            elif synth_type == 'supersaw':
                sig = synth_supersaw(freq, sustain)
            elif synth_type == 'superfm':
                sig = synth_superfm(freq, sustain)
            elif synth_type == 'noise':
                sig = synth_noise(sustain)
            else:
                sig = synth_sine(freq, sustain)

            sig = apply_envelope(sig, attack, release)
            if lpf_cut < SR / 2 - 1:
                sig = apply_lpf(sig, lpf_cut)
            if hpf_cut > 0:
                sig = apply_hpf(sig, hpf_cut)
            if delay_mix > 0:
                sig = apply_delay(sig, delay_time, delay_fb, delay_mix)

            place(buf, sig, t, gain)

    # Master processing
    buf = apply_lpf(buf, 10000)
    buf = np.tanh(buf * 0.85) * 0.75
    buf /= (np.max(np.abs(buf)) + 1e-8) * 0.85
    return buf


def compute_similarity(orig_path, synth_audio, sr=SR):
    """Compute real spectral similarity between original and synthesized audio."""
    y_orig, _ = librosa.load(orig_path, sr=sr)
    y_synth = synth_audio

    ml = min(len(y_orig), len(y_synth))
    y_orig, y_synth = y_orig[:ml], y_synth[:ml]

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
    rc = max(0, float(np.corrcoef(ro[:mr], rt[:mr])[0, 1]))

    weights = [0.35, 0.20, 0.15, 0.15, 0.15]
    metrics = [cos, cs, corr, ms, rc]
    labels = ['Band Cosine', 'Centroid', 'Band Corr', 'MFCC', 'RMS Corr']
    overall = sum(w*m for w, m in zip(weights, metrics))

    results = {
        'overall_similarity': round(overall, 4),
        'pass': overall >= 0.20,
        'metrics': {l: round(m, 4) for l, m in zip(labels, metrics)},
        'band_comparison': band_detail,
        'centroid': {'orig': round(ceno), 'synth': round(cent)},
    }

    # Print summary
    print(f"\n  {'Band':<18} {'Orig':>8} {'Synth':>8} {'Diff':>8}")
    print(f"  {'-'*44}")
    for nm in band_detail:
        b = band_detail[nm]
        print(f"  {nm:<18} {b['orig_db']:>7.1f}  {b['synth_db']:>7.1f}  {b['diff']:>+7.1f}")

    print(f"\n{'='*50}")
    for l, m, w in zip(labels, metrics, weights):
        print(f"  {l:<16} {m*100:>6.1f}%  {w*100:>4.0f}%  → {w*m*100:>5.1f}%")
    print(f"{'='*50}")
    print(f"  OVERALL: {overall*100:.1f}%  (≥20% → {'✅' if overall >= 0.2 else '❌'})")

    return results


if __name__ == '__main__':
    if len(sys.argv) < 3:
        print("Usage: python3 compare.py <original.wav> <tidal_params.json> [output_dir]")
        print("\ntidal_params.json format:")
        print(json.dumps({
            "_bpm": 120,
            "orbits": [{
                "note": "c2", "synth": "supersaw", "gain": 0.8,
                "lpf": 200, "sustain": 0.15, "attack": 0.003,
                "speed": 1, "interval": 1.0
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

    print("\nComputing similarity...")
    results = compute_similarity(orig_wav, synth, SR)

    results_path = os.path.join(out_dir, 'similarity.json')
    with open(results_path, 'w') as f:
        json.dump(results, f, indent=2)
    print(f"\nResults: {results_path}")
