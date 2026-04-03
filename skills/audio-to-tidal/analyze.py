#!/usr/bin/env python3
"""
Audio Spectrum Analyzer for TidalCycles generation.
Usage: python3 analyze.py <input.wav> [output_dir]

Outputs spectral data files for TidalCycles parameter mapping.
"""
import numpy as np
import librosa
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import json, csv, os, sys


def analyze(wav_path, out_dir=None):
    if out_dir is None:
        out_dir = os.path.join(os.path.dirname(wav_path), 'spectral_data')
    os.makedirs(out_dir, exist_ok=True)

    print(f"Loading: {wav_path}")
    y, sr = librosa.load(wav_path, sr=44100)
    dur = len(y) / sr

    # STFT (4096 for better freq resolution)
    S = np.abs(librosa.stft(y, n_fft=4096, hop_length=512))
    freqs = librosa.fft_frequencies(sr=sr, n_fft=4096)
    times = librosa.frames_to_time(np.arange(S.shape[1]), sr=sr, hop_length=512)
    avg_spec = np.mean(S, axis=1)

    # 11-band energy
    bands = [
        ('sub_20_40', 20, 40), ('sub_40_60', 40, 60),
        ('bass_60_100', 60, 100), ('bass_100_200', 100, 200),
        ('lowmid_200_400', 200, 400), ('mid_400_800', 400, 800),
        ('mid_800_1600', 800, 1600), ('upper_1600_3200', 1600, 3200),
        ('pres_3200_6400', 3200, 6400), ('bril_6400_12800', 6400, 12800),
        ('air_12800_20000', 12800, 20000),
    ]
    band_energy = {}
    for name, lo, hi in bands:
        li = np.searchsorted(freqs, lo)
        hi_i = np.searchsorted(freqs, min(hi, freqs[-1]))
        if hi_i > li:
            e = float(np.mean(S[li:hi_i, :] ** 2))
            band_energy[name] = {'hz': (lo, hi), 'energy': e, 'db': 10*np.log10(e+1e-10)}

    # Spectral features
    centroid = librosa.feature.spectral_centroid(y=y, sr=sr)[0]
    rolloff = librosa.feature.spectral_rolloff(y=y, sr=sr)[0]
    bandwidth = librosa.feature.spectral_bandwidth(y=y, sr=sr)[0]

    # MFCC
    mfccs = librosa.feature.mfcc(y=y, sr=sr, n_mfcc=13)

    # Tempo & beats
    tempo, beat_frames = librosa.beat.beat_track(y=y, sr=sr)
    tempo = float(np.atleast_1d(tempo)[0])
    beat_times = librosa.frames_to_time(beat_frames, sr=sr)

    # Onsets
    onset_frames = librosa.onset.onset_detect(y=y, sr=sr, backtrack=True)
    onset_times = librosa.frames_to_time(onset_frames, sr=sr)

    # RMS
    rms = librosa.feature.rms(y=y)[0]

    # Chroma
    chroma = librosa.feature.chroma_cqt(y=y, sr=sr)
    chroma_avg = np.mean(chroma, axis=1)
    note_names = ['C','C#','D','D#','E','F','F#','G','G#','A','A#','B']
    top_notes = [note_names[i] for i in np.argsort(chroma_avg)[::-1][:5]]

    # Spectral peaks
    from scipy.signal import find_peaks
    pk_idx, _ = find_peaks(avg_spec, height=np.max(avg_spec)*0.1, distance=10)
    peaks = sorted([(float(freqs[i]), float(avg_spec[i])) for i in pk_idx], key=lambda x: -x[1])

    # === Temporal Analysis (NEW) ===

    # 1. Onset interval pattern — rhythm signature
    onset_intervals = np.diff(onset_times) if len(onset_times) > 1 else np.array([])
    onset_interval_histogram = []
    if len(onset_intervals) > 0:
        bins = np.linspace(0, 2.0, 41)  # 50ms bins up to 2s
        hist, bin_edges = np.histogram(onset_intervals, bins=bins, density=True)
        onset_interval_histogram = [round(float(h), 4) for h in hist]

    # 2. Spectral flux — frame-to-frame spectral change (temporal timbre evolution)
    S_phase = np.abs(librosa.stft(y, n_fft=4096, hop_length=512))
    # Half-wave rectified difference
    flux = np.sum(np.maximum(0, np.diff(S_phase, axis=1)), axis=0)
    flux_norm = flux / (np.max(flux) + 1e-10)

    # 3. Temporal centroid — how energy distributes over time
    if len(rms) > 0:
        rms_norm = rms / (np.sum(rms) + 1e-10)
        temporal_centroid = float(np.sum(rms_norm * np.arange(len(rms))))
    else:
        temporal_centroid = 0.0

    # 4. Beat-aligned energy envelope
    if len(beat_times) > 2:
        beat_rms = []
        for i in range(len(beat_frames)):
            start = beat_frames[i]
            end = beat_frames[i+1] if i+1 < len(beat_frames) else len(rms)
            if end > start:
                beat_rms.append(float(np.mean(rms[start:end])))
        beat_rms = np.array(beat_rms)
        # Normalize per-beat envelope
        if np.max(beat_rms) > 0:
            beat_rms_norm = beat_rms / np.max(beat_rms)
        else:
            beat_rms_norm = beat_rms
    else:
        beat_rms = np.array([])
        beat_rms_norm = np.array([])

    # 5. Beat-aligned spectral centroid
    if len(beat_times) > 2:
        beat_centroid = []
        for i in range(len(beat_frames)):
            start = beat_frames[i]
            end = beat_frames[i+1] if i+1 < len(beat_frames) else len(centroid)
            if end > start:
                beat_centroid.append(float(np.mean(centroid[start:end])))
        beat_centroid = np.array(beat_centroid)
    else:
        beat_centroid = np.array([])

    # 6. Onset density over time (events per second, 0.5s windows)
    window_sec = 0.5
    num_windows = max(1, int(dur / window_sec) + 1)
    onset_density = np.zeros(num_windows)
    for t in onset_times:
        idx = min(int(t / window_sec), num_windows - 1)
        onset_density[idx] += 1

    # 7. Onset strength — mean onset detection function per beat
    onset_env = librosa.onset.onset_strength(y=y, sr=sr)
    if len(beat_frames) > 2:
        beat_onset_strength = []
        for i in range(len(beat_frames)):
            start = beat_frames[i]
            end = beat_frames[i+1] if i+1 < len(beat_frames) else len(onset_env)
            if end > start:
                beat_onset_strength.append(float(np.mean(onset_env[start:end])))
    else:
        beat_onset_strength = []

    # === Export CSVs ===
    with open(os.path.join(out_dir, 'band_energy.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['band', 'hz_low', 'hz_high', 'avg_energy', 'energy_db'])
        for name, (lo, hi) in bands:
            b = band_energy.get(name, {'energy': 0, 'db': -100})
            w.writerow([name, lo, hi, f'{b["energy"]:.8f}', f'{b["db"]:.2f}'])

    with open(os.path.join(out_dir, 'spectral_features.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        ft = librosa.frames_to_time(np.arange(len(centroid)), sr=sr)
        w.writerow(['time_s', 'centroid_hz', 'rolloff_hz', 'bandwidth_hz'])
        for i in range(len(centroid)):
            w.writerow([f'{ft[i]:.3f}', f'{centroid[i]:.1f}', f'{rolloff[i]:.1f}', f'{bandwidth[i]:.1f}'])

    with open(os.path.join(out_dir, 'mfcc.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        mt = librosa.frames_to_time(np.arange(mfccs.shape[1]), sr=sr)
        w.writerow(['time_s'] + [f'mfcc_{i}' for i in range(13)])
        for j in range(mfccs.shape[1]):
            w.writerow([f'{mt[j]:.3f}'] + [f'{mfccs[i,j]:.4f}' for i in range(13)])

    with open(os.path.join(out_dir, 'chroma.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['note', 'avg_energy'])
        for i, n in enumerate(note_names):
            w.writerow([n, f'{chroma_avg[i]:.6f}'])

    with open(os.path.join(out_dir, 'avg_spectrum.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['freq_hz', 'magnitude'])
        for i in range(len(freqs)):
            w.writerow([f'{freqs[i]:.1f}', f'{avg_spec[i]:.6f}'])

    # === Temporal CSV exports (NEW) ===
    with open(os.path.join(out_dir, 'spectral_flux.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['time_s', 'flux'])
        for i in range(len(flux_norm)):
            w.writerow([f'{times[i]:.3f}', f'{flux_norm[i]:.6f}'])

    with open(os.path.join(out_dir, 'beat_energy.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['beat_index', 'time_s', 'rms', 'rms_norm', 'centroid_hz'])
        for i in range(len(beat_rms)):
            bt = beat_times[i] if i < len(beat_times) else 0
            bc = beat_centroid[i] if i < len(beat_centroid) else 0
            w.writerow([i, f'{bt:.3f}', f'{beat_rms[i]:.6f}',
                        f'{beat_rms_norm[i]:.4f}', f'{bc:.1f}'])

    with open(os.path.join(out_dir, 'onset_density.csv'), 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['window_s', 'onset_count'])
        for i in range(len(onset_density)):
            w.writerow([f'{i * window_sec:.1f}', f'{onset_density[i]:.0f}'])

    # === Summary JSON ===
    summary = {
        'duration_s': round(dur, 2), 'sample_rate': sr,
        'detected_tempo_bpm': round(tempo, 1), 'num_beats': len(beat_times),
        'num_onsets': len(onset_times),
        'avg_centroid_hz': round(float(np.mean(centroid)), 1),
        'avg_rolloff_hz': round(float(np.mean(rolloff)), 1),
        'avg_bandwidth_hz': round(float(np.mean(bandwidth)), 1),
        'band_energy_db': {k: round(v['db'], 2) for k, v in band_energy.items()},
        'dominant_band': max(band_energy, key=lambda k: band_energy[k]['energy']),
        'top_notes': top_notes,
        'note_scores': {note_names[i]: round(float(chroma_avg[i]), 4) for i in range(12)},
        'spectral_peaks': [(round(f, 1), librosa.hz_to_note(f) if f > 20 else '-') for f, _ in peaks[:10]],
        'mfcc_means': [round(float(np.mean(mfccs[i])), 2) for i in range(13)],
        'rms_mean': round(float(np.mean(rms)), 6),
        'rms_std': round(float(np.std(rms)), 6),
        # === Temporal features (NEW) ===
        'temporal': {
            'temporal_centroid_frame': round(temporal_centroid, 2),
            'onset_interval_mean_s': round(float(np.mean(onset_intervals)), 4) if len(onset_intervals) > 0 else None,
            'onset_interval_std_s': round(float(np.std(onset_intervals)), 4) if len(onset_intervals) > 0 else None,
            'onset_interval_cv': round(float(np.std(onset_intervals) / (np.mean(onset_intervals) + 1e-10)), 3) if len(onset_intervals) > 0 else None,
            'onset_interval_histogram': onset_interval_histogram,
            'spectral_flux_mean': round(float(np.mean(flux_norm)), 4),
            'spectral_flux_std': round(float(np.std(flux_norm)), 4),
            'beat_rms_mean': round(float(np.mean(beat_rms_norm)), 4) if len(beat_rms_norm) > 0 else None,
            'beat_rms_std': round(float(np.std(beat_rms_norm)), 4) if len(beat_rms_norm) > 0 else None,
            'beat_centroid_mean': round(float(np.mean(beat_centroid)), 1) if len(beat_centroid) > 0 else None,
            'beat_centroid_std': round(float(np.std(beat_centroid)), 1) if len(beat_centroid) > 0 else None,
            'onset_density_mean': round(float(np.mean(onset_density)), 3),
            'onset_density_std': round(float(np.std(onset_density)), 3),
            'beat_onset_strength': [round(float(x), 2) for x in beat_onset_strength],
        },
    }
    with open(os.path.join(out_dir, 'summary.json'), 'w') as f:
        json.dump(summary, f, indent=2, default=str)

    # === Visualization ===
    fig, axes = plt.subplots(6, 1, figsize=(14, 22))
    S_db = librosa.amplitude_to_db(S, ref=np.max)
    img = librosa.display.specshow(S_db, sr=sr, hop_length=512, x_axis='time', y_axis='log', ax=axes[0])
    axes[0].set_ylim(20, 15000); axes[0].set_title('Spectrogram')
    fig.colorbar(img, ax=axes[0], format='%+2.0f dB')

    axes[1].semilogx(freqs[freqs>0], 20*np.log10(avg_spec[freqs>0]+1e-10), linewidth=0.8)
    for f, m in peaks[:8]:
        axes[1].axvline(f, color='r', alpha=0.4, linewidth=0.5)
    axes[1].set_xlim(20, 20000); axes[1].set_xlabel('Hz'); axes[1].set_ylabel('dB')
    axes[1].set_title('Average Spectrum'); axes[1].grid(True, alpha=0.3)

    ft = librosa.frames_to_time(np.arange(len(centroid)), sr=sr)
    axes[2].plot(ft, centroid, label='Centroid', alpha=0.8)
    axes[2].plot(ft, rolloff, label='Rolloff', alpha=0.6)
    axes[2].set_xlabel('s'); axes[2].set_ylabel('Hz'); axes[2].set_title('Spectral Features')
    axes[2].legend(); axes[2].grid(True, alpha=0.3)

    rt = librosa.frames_to_time(np.arange(len(rms)), sr=sr)
    axes[3].plot(rt, rms, label='RMS')
    for bt in beat_times: axes[3].axvline(bt, color='r', alpha=0.3, linewidth=0.5)
    axes[3].set_xlabel('s'); axes[3].set_ylabel('RMS')
    axes[3].set_title(f'RMS Envelope (Tempo: {tempo:.1f} BPM)'); axes[3].legend(); axes[3].grid(True, alpha=0.3)

    # NEW: Spectral flux
    axes[4].plot(times[:len(flux_norm)], flux_norm, color='purple', alpha=0.8)
    for ot in onset_times: axes[4].axvline(ot, color='orange', alpha=0.3, linewidth=0.5)
    axes[4].set_xlabel('s'); axes[4].set_ylabel('Flux')
    axes[4].set_title('Spectral Flux + Onsets'); axes[4].grid(True, alpha=0.3)

    # NEW: Beat-aligned features
    if len(beat_rms_norm) > 0:
        bx = np.arange(len(beat_rms_norm))
        ax_rms = axes[5]
        ax_cen = ax_rms.twinx()
        ax_rms.bar(bx, beat_rms_norm, alpha=0.5, color='steelblue', label='Beat RMS')
        if len(beat_centroid) > 0:
            ax_cen.plot(bx[:len(beat_centroid)], beat_centroid, color='tomato',
                        marker='o', markersize=3, label='Beat Centroid')
        ax_rms.set_xlabel('Beat Index'); ax_rms.set_ylabel('RMS (norm)', color='steelblue')
        ax_cen.set_ylabel('Centroid Hz', color='tomato')
        ax_rms.set_title('Beat-Aligned Energy & Centroid')
        lines1, labels1 = ax_rms.get_legend_handles_labels()
        lines2, labels2 = ax_cen.get_legend_handles_labels()
        ax_rms.legend(lines1+lines2, labels1+labels2, loc='upper right')
        ax_rms.grid(True, alpha=0.3)
    else:
        axes[5].text(0.5, 0.5, 'No beats detected', ha='center', va='center',
                     transform=axes[5].transAxes, fontsize=14)
        axes[5].set_title('Beat-Aligned Features')

    plt.tight_layout()
    plt.savefig(os.path.join(out_dir, 'spectral_analysis.png'), dpi=150, bbox_inches='tight')
    plt.close()

    print(f"\nAnalysis complete. Output: {out_dir}/")
    print(f"  Tempo: {tempo:.1f} BPM")
    print(f"  Onsets: {len(onset_times)}")
    print(f"  Top notes: {top_notes}")
    print(f"  Peaks: {summary['spectral_peaks'][:5]}")
    print(f"  Dominant: {summary['dominant_band']}")
    print(f"  Onset interval: {summary['temporal']['onset_interval_mean_s']:.3f}s ± {summary['temporal']['onset_interval_std_s']:.3f}s (CV={summary['temporal']['onset_interval_cv']:.2f})")
    print(f"  Spectral flux: {summary['temporal']['spectral_flux_mean']:.3f} ± {summary['temporal']['spectral_flux_std']:.3f}")
    return summary


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print("Usage: python3 analyze.py <input.wav> [output_dir]")
        sys.exit(1)
    wav = sys.argv[1]
    out = sys.argv[2] if len(sys.argv) > 2 else None
    analyze(wav, out)
