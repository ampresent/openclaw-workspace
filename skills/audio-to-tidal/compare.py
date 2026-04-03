#!/usr/bin/env python3
"""
TidalCycles Output Synthesizer & Similarity Comparator.
Usage: python3 compare.py <original.wav> <tidal_params.json> [output_dir]

Synthesizes expected TidalCycles output from SuperDirt parameters,
then computes real spectral similarity vs original audio.

Metrics (8 total, frequency + time domain):
  Frequency: Band Cosine, Centroid, Band Correlation, MFCC
  Time:      RMS Correlation, Onset Pattern, Temporal Centroid, Beat Energy
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

        try:
            freq = librosa.note_to_hz(note)
        except:
            freq = 130.81

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

    buf = apply_lpf(buf, 10000)
    buf = np.tanh(buf * 0.85) * 0.75
    buf /= (np.max(np.abs(buf)) + 1e-8) * 0.85
    return buf


# === Time-domain similarity functions (NEW) ===

def compute_onset_pattern_similarity(y_orig, y_synth, sr=SR):
    """
    Compare onset interval distributions — captures rhythm pattern similarity.
    Uses histogram correlation of inter-onset intervals + density correlation.
    """
    env_o = librosa.onset.onset_strength(y=y_orig, sr=sr)
    env_s = librosa.onset.onset_strength(y=y_synth, sr=sr)

    onsets_o = librosa.onset.onset_detect(y=y_orig, sr=sr, backtrack=False)
    onsets_s = librosa.onset.onset_detect(y=y_synth, sr=sr, backtrack=False)
    times_o = librosa.frames_to_time(onsets_o, sr=sr)
    times_s = librosa.frames_to_time(onsets_s, sr=sr)

    intervals_o = np.diff(times_o) if len(times_o) > 1 else np.array([0])
    intervals_s = np.diff(times_s) if len(times_s) > 1 else np.array([0])

    # Histogram correlation
    bins = np.linspace(0, 2.0, 41)
    hist_o, _ = np.histogram(intervals_o, bins=bins, density=True)
    hist_s, _ = np.histogram(intervals_s, bins=bins, density=True)

    if np.std(hist_o) > 1e-10 and np.std(hist_s) > 1e-10:
        hist_corr = float(np.corrcoef(hist_o, hist_s)[0, 1])
    else:
        hist_corr = 0.0

    # Onset density correlation (0.5s windows)
    dur = max(len(y_orig), len(y_synth)) / sr
    win = 0.5
    nw = max(1, int(dur / win) + 1)

    def density(times, n):
        d = np.zeros(n)
        for t in times:
            d[min(int(t / win), n - 1)] += 1
        return d

    den_o = density(times_o, nw)
    den_s = density(times_s, nw)
    ml = min(len(den_o), len(den_s))
    if ml > 1 and np.std(den_o[:ml]) > 1e-10 and np.std(den_s[:ml]) > 1e-10:
        den_corr = float(np.corrcoef(den_o[:ml], den_s[:ml])[0, 1])
    else:
        den_corr = 0.0

    # Onset strength envelope correlation (compressed into 100 bins)
    def compress_env(env, n_bins=100):
        if len(env) == 0:
            return np.zeros(n_bins)
        chunk = max(1, len(env) // n_bins)
        return np.array([np.mean(env[i*chunk:(i+1)*chunk]) for i in range(n_bins)])[:n_bins]

    comp_o = compress_env(env_o)
    comp_s = compress_env(env_s)
    if np.std(comp_o) > 1e-10 and np.std(comp_s) > 1e-10:
        env_corr = float(np.corrcoef(comp_o, comp_s)[0, 1])
    else:
        env_corr = 0.0

    # Combined onset similarity
    similarity = 0.4 * max(0, hist_corr) + 0.3 * max(0, den_corr) + 0.3 * max(0, env_corr)

    return similarity, {
        'histogram_corr': round(hist_corr, 4),
        'density_corr': round(den_corr, 4),
        'onset_env_corr': round(env_corr, 4),
    }


def compute_temporal_centroid_similarity(y_orig, y_synth, sr=SR):
    """
    Compare when energy is concentrated in time — captures attack/release
    structure and overall temporal envelope shape.
    """
    n_bins = 100  # resample both to fixed resolution

    def temporal_profile(y, n):
        rms = librosa.feature.rms(y=y, hop_length=512)[0]
        if len(rms) == 0:
            return np.zeros(n)
        chunk = max(1, len(rms) // n)
        profile = np.array([np.mean(rms[i*chunk:(i+1)*chunk]) for i in range(n)])[:n]
        if np.sum(profile) > 0:
            profile /= np.sum(profile)
        return profile

    prof_o = temporal_profile(y_orig, n_bins)
    prof_s = temporal_profile(y_synth, n_bins)

    # Correlation of temporal profiles
    if np.std(prof_o) > 1e-10 and np.std(prof_s) > 1e-10:
        corr = float(np.corrcoef(prof_o, prof_s)[0, 1])
    else:
        corr = 0.0

    # Centroid of temporal profile (when is the energy centered?)
    tc_o = float(np.sum(prof_o * np.arange(n_bins)))
    tc_s = float(np.sum(prof_s * np.arange(n_bins)))
    tc_sim = 1.0 - abs(tc_o - tc_s) / n_bins

    # Energy distribution shape (variance of profile = spread)
    spread_o = float(np.std(np.arange(n_bins) * prof_o / (np.sum(prof_o) + 1e-10)))
    spread_s = float(np.std(np.arange(n_bins) * prof_s / (np.sum(prof_s) + 1e-10)))
    spread_sim = 1.0 - abs(spread_o - spread_s) / (max(spread_o, spread_s) + 1e-10)

    similarity = 0.4 * max(0, corr) + 0.3 * max(0, tc_sim) + 0.3 * max(0, spread_sim)

    return similarity, {
        'profile_corr': round(corr, 4),
        'temporal_centroid_sim': round(tc_sim, 4),
        'spread_sim': round(spread_sim, 4),
    }


def compute_beat_energy_similarity(y_orig, y_synth, sr=SR):
    """
    Compare beat-aligned energy and spectral centroid — captures per-beat
    dynamic structure and timbre variation across the rhythm grid.
    """
    _, beats_o = librosa.beat.beat_track(y=y_orig, sr=sr)
    _, beats_s = librosa.beat.beat_track(y=y_synth, sr=sr)

    beats_o = np.atleast_1d(beats_o)
    beats_s = np.atleast_1d(beats_s)

    rms_o = librosa.feature.rms(y=y_orig, hop_length=512)[0]
    rms_s = librosa.feature.rms(y=y_synth, hop_length=512)[0]

    def beat_rms_profile(beats, rms, max_beats=64):
        if len(beats) < 3:
            return np.array([])
        prof = []
        for i in range(min(len(beats) - 1, max_beats)):
            s, e = beats[i], beats[i+1]
            if e > s:
                prof.append(float(np.mean(rms[s:e])))
        return np.array(prof)

    brms_o = beat_rms_profile(beats_o, rms_o)
    brms_s = beat_rms_profile(beats_s, rms_s)

    # Beat energy correlation
    ml = min(len(brms_o), len(brms_s))
    if ml > 2 and np.std(brms_o[:ml]) > 1e-10 and np.std(brms_s[:ml]) > 1e-10:
        beat_corr = float(np.corrcoef(brms_o[:ml], brms_s[:ml])[0, 1])
    else:
        beat_corr = 0.0

    # Beat centroid correlation
    cent_o = librosa.feature.spectral_centroid(y=y_orig, sr=sr)[0]
    cent_s = librosa.feature.spectral_centroid(y=y_synth, sr=sr)[0]

    def beat_cent_profile(beats, cent, max_beats=64):
        if len(beats) < 3:
            return np.array([])
        prof = []
        for i in range(min(len(beats) - 1, max_beats)):
            s, e = beats[i], beats[i+1]
            if e > s:
                prof.append(float(np.mean(cent[s:e])))
        return np.array(prof)

    bc_o = beat_cent_profile(beats_o, cent_o)
    bc_s = beat_cent_profile(beats_s, cent_s)

    ml_c = min(len(bc_o), len(bc_s))
    if ml_c > 2 and np.std(bc_o[:ml_c]) > 1e-10 and np.std(bc_s[:ml_c]) > 1e-10:
        cent_corr = float(np.corrcoef(bc_o[:ml_c], bc_s[:ml_c])[0, 1])
    else:
        cent_corr = 0.0

    # Per-beat RMS variance ratio (dynamic contrast)
    var_o = float(np.std(brms_o)) if len(brms_o) > 0 else 0
    var_s = float(np.std(brms_s)) if len(brms_s) > 0 else 0
    var_sim = 1.0 - abs(var_o - var_s) / (max(var_o, var_s) + 1e-10)

    similarity = 0.4 * max(0, beat_corr) + 0.35 * max(0, cent_corr) + 0.25 * max(0, var_sim)

    return similarity, {
        'beat_rms_corr': round(beat_corr, 4),
        'beat_centroid_corr': round(cent_corr, 4),
        'dynamic_contrast_sim': round(var_sim, 4),
    }


def compute_similarity(orig_path, synth_audio, sr=SR):
    """Compute spectral + temporal similarity between original and synthesized audio."""
    y_orig, _ = librosa.load(orig_path, sr=sr)
    y_synth = synth_audio

    ml = min(len(y_orig), len(y_synth))
    y_orig, y_synth = y_orig[:ml], y_synth[:ml]

    # === Frequency domain metrics (existing) ===
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

    # Existing time metric
    ro = librosa.feature.rms(y=y_orig)[0]
    rt = librosa.feature.rms(y=y_synth)[0]
    mr = min(len(ro), len(rt))
    rms_corr = max(0, float(np.corrcoef(ro[:mr], rt[:mr])[0, 1])) if mr > 1 else 0

    # === NEW time-domain metrics ===
    onset_sim, onset_detail = compute_onset_pattern_similarity(y_orig, y_synth, sr)
    tc_sim, tc_detail = compute_temporal_centroid_similarity(y_orig, y_synth, sr)
    beat_sim, beat_detail = compute_beat_energy_similarity(y_orig, y_synth, sr)

    # === Weighted overall score ===
    # Frequency: 25% | Time: 35% (was 15%)
    # This reflects that for TidalCycles patterns, temporal structure matters more
    labels = ['Band Cosine', 'Centroid', 'Band Corr', 'MFCC',
              'RMS Corr', 'Onset Pattern', 'Temporal Envelope', 'Beat Structure']
    weights = [0.15, 0.10, 0.10, 0.10,   # freq: 45%
               0.10, 0.20, 0.15, 0.20]    # time: 55%
    metrics = [cos, cs, corr, ms, rms_corr, onset_sim, tc_sim, beat_sim]

    overall = sum(w * m for w, m in zip(weights, metrics))

    results = {
        'overall_similarity': round(overall, 4),
        'pass': overall >= 0.20,
        'metrics': {l: round(m, 4) for l, m in zip(labels, metrics)},
        'frequency_domain': {
            'band_comparison': band_detail,
            'centroid': {'orig': round(ceno), 'synth': round(cent)},
        },
        'time_domain': {
            'onset_pattern': onset_detail,
            'temporal_centroid': tc_detail,
            'beat_energy': beat_detail,
        },
    }

    # Print summary
    print(f"\n  {'Band':<18} {'Orig':>8} {'Synth':>8} {'Diff':>8}")
    print(f"  {'-'*44}")
    for nm in band_detail:
        b = band_detail[nm]
        print(f"  {nm:<18} {b['orig_db']:>7.1f}  {b['synth_db']:>7.1f}  {b['diff']:>+7.1f}")

    print(f"\n{'='*60}")
    print(f"  FREQUENCY DOMAIN")
    for l, m, w in zip(labels[:4], metrics[:4], weights[:4]):
        print(f"    {l:<16} {m*100:>6.1f}%  {w*100:>4.0f}%  → {w*m*100:>5.1f}%")
    print(f"\n  TIME DOMAIN")
    for l, m, w in zip(labels[4:], metrics[4:], weights[4:]):
        print(f"    {l:<16} {m*100:>6.1f}%  {w*100:>4.0f}%  → {w*m*100:>5.1f}%")
    print(f"{'='*60}")
    freq_score = sum(w*m for w, m in zip(weights[:4], metrics[:4]))
    time_score = sum(w*m for w, m in zip(weights[4:], metrics[4:]))
    print(f"  FREQ SUBTOTAL: {freq_score*100:.1f}%  TIME SUBTOTAL: {time_score*100:.1f}%")
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

    print("\nComputing similarity (frequency + time domain)...")
    results = compute_similarity(orig_wav, synth, SR)

    results_path = os.path.join(out_dir, 'similarity.json')
    with open(results_path, 'w') as f:
        json.dump(results, f, indent=2)
    print(f"\nResults: {results_path}")
