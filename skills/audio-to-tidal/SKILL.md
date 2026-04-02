# Audio → TidalCycles 频谱模仿

分析音频文件的频谱特征，生成 TidalCycles 代码来模仿目标音频，并计算频谱相似度。

## 触发条件

当用户想要用 TidalCycles 模仿/复现某个音频时使用，例如：
- "用 TidalCycles 模仿这段音频"
- "分析这个音乐，生成 Tidal 代码"
- "把这个 MP3/WAV 转成 TidalCycles"
- "用 Tidal 模仿 Deepchord / dub techno / ambient 等风格"

## 依赖

- Python 3.8+
- `librosa`、`numpy`、`scipy`、`soundfile`、`matplotlib`（pip install）
- `ffmpeg`（用于音频格式转换）
- TidalCycles + SuperDirt（用于最终播放，不在本 skill 内）

## 工作流程

### 1. 获取音频

支持以下来源：
- **用户上传文件**（MP4/WAV/MP3 等）
- **本地文件路径**
- **URL 下载**（需要网络可达）

如果是视频文件（MP4），用 ffmpeg 提取音频：
```bash
ffmpeg -y -i input.mp4 -vn -acodec pcm_s16le -ar 44100 -ac 1 output.wav
```

统一输出为 44100Hz mono WAV。

### 2. 深度频谱分析

运行 Python 分析脚本，提取以下特征：

#### 必须提取的数据
| 特征 | 用途 | 方法 |
|------|------|------|
| **BPM / 节拍** | 设置 `setcps` | `librosa.beat.beat_track` |
| **频谱峰值频率** | 确定音高 / note 参数 | `scipy.signal.find_peaks` on avg spectrum |
| **频带能量分布** | 11 个频段的 dB 值 | 分频段计算 STFT 能量 |
| **频谱质心** | 评估整体亮度 | `librosa.feature.spectral_centroid` |
| **Chroma 音调** | 确定和弦/音阶 | `librosa.feature.chroma_cqt` |
| **MFCC 倒谱系数** | 音色特征 | `librosa.feature.mfcc` |
| **Onset 起音** | 节奏密度 | `librosa.onset.onset_detect` |
| **RMS 包络** | 动态变化 | `librosa.feature.rms` |

#### 频带划分（11 段）
```
sub_20_40     20-40 Hz      极低频
sub_40_60     40-60 Hz      Sub-bass
bass_60_100   60-100 Hz     Bass 基频
bass_100_200  100-200 Hz    Bass 泛音
lowmid_200_400  200-400 Hz   低中频
mid_400_800   400-800 Hz    中频
mid_800_1600  800-1600 Hz   中高频
upper_1600_3200  1600-3200 Hz 上中频
pres_3200_6400  3200-6400 Hz 临场感
bril_6400_12800  6400-12800 Hz 明亮度
air_12800_20000  12800-20000 Hz 空气感
```

#### 导出文件
所有数据导出到 `<工作目录>/spectral_data/`：
- `full_spectrum.csv` — 完整时频矩阵
- `avg_spectrum.csv` — 平均频谱
- `band_energy.csv` — 11 频段能量
- `spectral_features.csv` — 质心/滚降/带宽时序
- `mfcc.csv` — MFCC 系数
- `beats.csv` — 节拍时间点
- `rms_envelope.csv` — RMS 包络
- `chroma.csv` — 12 音级能量
- `summary.json` — 汇总 JSON
- `spectral_analysis.png` — 可视化图表

### 3. 生成 TidalCycles 代码

根据分析结果，生成 `.tidal` 文件。关键映射规则：

#### 节拍
```
原始 BPM → setcps (BPM/60/4)
```
- 如果 BPM 检测不准（如 ambient 无明显节拍），使用 60-120 的合理值
- 对于 half-time 风格，仍用实际 BPM，节奏靠 pattern 控制

#### 音高 → Note
- 取频谱峰值频率，转换为音名：`librosa.hz_to_note(freq)`
- 主峰值 → kick/bass 的 note
- 次峰值 → 和弦/旋律的 note
- 结合 chroma 确认调性

#### 频带能量 → 轨道分配
| 频段能量特征 | 对应 SuperDirt 轨道 | 关键参数 |
|-------------|-------------------|---------|
| sub_bass 主导 | d1 kick + d2 sub-bass | lpf 100-200, gain 0.7-0.9 |
| bass 主导 | d3 bass layer | lpf 200-600, supersaw/superhex |
| mid 有能量 | d4 chord stabs | lpf 800-1800, delay |
| presence 有能量 | d5/d6 高频元素 | lpf 2000-5000 |
| air 有能量 | d7 noise/hats | hpf 4000+, superwhite |

#### 合成器选择
| SuperDirt 合成器 | 音色特征 | 适用场景 |
|-----------------|---------|---------|
| `superhex` | 纯正弦波，无泛音 | kick, sub-bass, 纯低频 |
| `supersaw` | 失谐锯齿波，丰富泛音 | bass, pad, 需要谐波的元素 |
| `superfm` | FM 合成，金属感 | 和弦 stabs, 铃声质感 |
| `superwhite` | 白噪声 | 纹理, hi-hat, 大气 |

**重要**: `superhex` 几乎没有谐波，如果目标音频在中高频有能量，必须用 `supersaw` 或 `superfm`。

#### 滤波器映射
- 频谱滚降频率 → `lpf` 截止频率
- 频谱质心 → 整体亮度参考
- 如果低频主导且滚降低 → 整体 lpf 偏低 (600-1200)
- 如果有 presence/brilliance 能量 → 保持 lpf > 2000

#### 效果器
- **Delay**: dub techno 风格用 `delay 0.5-0.8`, `delaytime (1/4)` 或 `(3/16)`
- **Reverb**: 大气风格用 `room 0.6-0.9`, `sz 0.7-0.9`
- **Degrade**: 密集节奏用 `degradeBy 0.3-0.6` 稀疏化

### 4. 频谱相似度计算

#### 方法：合成对比法（非估算）

**核心原则**: 不要凭经验估算 TidalCycles 输出，而是用 Python 合成 SuperDirt 的预期输出，然后进行真实的频谱对比。

步骤：
1. 根据 TidalCycles 代码中的合成器参数，用 Python 合成等效音频
2. 对合成音频做相同的频谱分析
3. 计算两个音频的真实相似度

#### 合成映射
| SuperDirt | Python 等效 |
|-----------|------------|
| superhex | `np.sin(2*np.pi*freq*t)` |
| supersaw | 多个 detuned sine + 谐波 |
| superfm | `np.sin(2πft + I*np.sin(2πf_m*t))` |
| superwhite | `np.random.randn()` |
| lpf | `scipy.signal.butter` low-pass |
| hpf | `scipy.signal.butter` high-pass |
| delay | 延迟叠加 |
| gain | 乘法 |
| sustain/attack/release | 包络 |

#### 相似度指标（5 项加权）
| 指标 | 权重 | 方法 |
|------|------|------|
| Band Energy Cosine | 35% | 11 频段线性能量的余弦相似度 |
| Spectral Centroid | 20% | 1 - \|orig - tidal\| / orig |
| Band Correlation | 15% | dB 向量的 Pearson 相关系数 |
| MFCC Similarity | 15% | 1 - \|\|m1-m2\|\| / (\|\|m1\|\|+\|\|m2\|\|) |
| RMS Correlation | 15% | RMS 包络的时间相关 |

#### 迭代优化
如果相似度 < 40% 或某频段差异 > 10 dB：
1. 定位最大差异频段
2. 检查对应的 TidalCycles 轨道参数
3. 调整 gain / lpf / 合成器选择
4. 重新合成对比
5. 最多迭代 3-5 轮

### 5. 输出文件

```
temp/
├── audio_source.wav          # 转换后的音频
├── spectral_data/            # 频谱分析数据
│   ├── summary.json
│   ├── band_energy.csv
│   ├── spectral_analysis.png
│   └── ...
├── audio_tidal.tidal         # TidalCycles 代码
├── tidal_synthesis.wav       # Python 合成的对比音频
└── similarity_results.json   # 相似度结果
```

## 常见问题

### Q: 相似度估算很高但听起来完全不像？

A: 之前的"估算"方法是用分析值直接比较分析值，存在循环论证。**必须用合成对比法**——实际生成音频再比较频谱。

### Q: SuperDirt 的 superhex 为什么缺少中高频？

A: `superhex` 是纯正弦波，没有谐波。如果目标音频在 800-3200 Hz 有明显能量，
   必须用 `supersaw`（泛音丰富）或 `superfm`，不能只用 `superhex`。

### Q: Kick 的音高偏了怎么办？

A: pitch sweep 起始频率会影响频谱峰值位置。如果目标峰值在 65 Hz，
   sweep 应从 80-100 Hz 开始（不要从 150 Hz），衰减目标设为精确频率。

### Q: 频段差异 > 15 dB 怎么调？

A: 常见原因和修复：
   - sub_20_40/sub_40_60 过高 → 检查是否有 C1 sub drone，去掉或降低
   - mid_800_1600 过低 → 换用 supersaw/superfm，提高 lpf
   - upper_1600_3200 过低 → 添加高频 shimmer 轨道，降低 hpf
   - brilliance 过高 → 降低 noise gain，提高 hpf

## 注意事项

- 网络受限时无法下载外部音频，需用户提供文件或改用 Python 合成目标音频
- TidalCycles 代码需要 SuperDirt 音色库才能播放，本 skill 不负责音频渲染
- 相似度 ≥ 20% 即为合格（频谱形态匹配），≥ 50% 为良好，≥ 70% 为优秀
- 每次迭代调整不宜过大（gain ±0.1, lpf ±200 Hz）
