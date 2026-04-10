# Audio → TidalCycles 频谱模仿 (v2)

分析音频文件的频谱特征（频域 + 时域 + 谐波），生成 TidalCycles 代码来模仿目标音频，并计算多维相似度。

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

### 2. 深度分析（v2 扩展）

运行 `analyze.py`，提取以下特征：

#### 频域特征
| 特征 | 用途 |
|------|------|
| BPM / 节拍 | 设置 `setcps` |
| 频谱峰值频率 | 确定音高 / note 参数 |
| 11 频带能量分布 | 轨道频段分配 |
| 频谱质心 | 整体亮度 |
| Chroma 音调 | 和弦/音阶 |
| MFCC 倒谱系数 | 音色特征 |

#### 时域特征（v2 新增）
| 特征 | 用途 |
|------|------|
| Onset interval histogram | 节奏模式签名 |
| Spectral flux | 时变音色演化 |
| Beat-aligned RMS/centroid | 逐拍能量/亮度动态 |
| Onset density | 起音密度分布 |
| Temporal centroid | 能量时间分布 |

#### 谐波特征（v2 新增）
| 特征 | 用途 |
|------|------|
| Harmonic ratio | 谐波能量占比 → 饱和程度 |
| Saturation ratio | 高频/低频能量比 → 过载类型 |
| Resonance peaks | 共振滤波器检测 |
| Filter sweep | CV 变化 → LFO 调制检测 |

#### 延迟特征（v2 新增）
| 特征 | 用途 |
|------|------|
| Decay rate | 衰减速率 → feedback 值 |
| Reverb tail | 混响时长 |
| Delay character | dub/short/medium → 延迟风格 |

#### 导出文件
所有数据导出到 `<工作目录>/spectral_data/`：
- `summary.json` — 汇总 JSON（含 v2 新增字段）
- `band_energy.csv` — 11 频段能量
- `spectral_features.csv` — 质心/滚降/带宽时序
- `mfcc.csv` — MFCC 系数
- `chroma.csv` — 12 音级能量
- `spectral_flux.csv` — 频谱通量
- `beat_energy.csv` — 逐拍能量/质心
- `onset_density.csv` — 起音密度
- `avg_spectrum.csv` — 平均频谱
- `spectral_analysis.png` — 7 面板可视化

### 3. 生成 TidalCycles 代码

v2 支持两种参数格式：

#### v2 格式：信号链（推荐）

信号链架构更适合需要音色塑造的音乐（dub techno、ambient、industrial 等）：

```json
{
  "_bpm": 123,
  "chains": [
    {
      "source": {"notes": ["g2", "a#2", "d3"], "synth": "supersaw", "detune": 1.006},
      "filter": {"lpf": 350, "resonance": 0.75, "lfo_rate": 0.12, "lfo_depth": 0.8},
      "saturate": {"type": "tape", "drive": 2.8},
      "delay": {"time": 0.366, "feedback": 0.65, "lpf": 1800, "mix": 0.45},
      "reverb": {"size": 0.7, "damp": 0.4, "mix": 0.3},
      "pattern": {"speed": 1, "interval": 1.0, "degrade": 0.08, "sustain": 0.2, "attack": 0.003, "release": 0.1},
      "gain": 0.85
    }
  ]
}
```

每个 chain 是一条完整的信号处理链路：
```
source → filter → saturate → delay → reverb → output
```

#### v1 格式：轨道（向后兼容）

```json
{
  "_bpm": 120,
  "orbits": [
    {
      "notes": ["g2", "a#2"], "synth": "supersaw", "gain": 0.8,
      "lpf": 200, "sustain": 0.15,
      "saturation": 2.0, "sattype": "tape",
      "delay": 0.4, "delaytime": 0.366, "delayfeedback": 0.7
    }
  ]
}
```

#### 滤波器参数

| 参数 | 类型 | 说明 |
|------|------|------|
| `lpf` | Hz | 低通截止频率 |
| `hpf` | Hz | 高通截止频率 |
| `resonance` | 0-0.99 | 共振量（dub techno 核心参数） |
| `lfo_rate` | Hz | LFO 调制截止频率的速度 |
| `lfo_depth` | 0-1 | LFO 调制深度 |

#### 饱和参数

| 类型 | 特征 | 适用场景 |
|------|------|---------|
| `tape` | 偶次谐波，温暖 | dub techno, ambient |
| `tube` | 奇次谐波，厚重 | industrial, noise |
| `wavefolder` | 极端泛音，金属感 | experimental |

#### 延迟参数

| 参数 | 说明 |
|------|------|
| `time` | 延迟时间（秒） |
| `feedback` | 反馈量 0-0.95（dub 用 0.5-0.8） |
| `lpf` | 每次重复的 LPF 截止（越低越暗） |
| `mix` | 干湿比 0-1 |

dub delay 的核心：**每次回声通过 LPF 变暗**，模拟磁带延迟。

#### 合成器选择
| 合成器 | 音色特征 | 适用场景 |
|--------|---------|---------|
| `sine` | 纯净，无泛音 | sub-bass, 简单元素 |
| `supersaw` | 失谐锯齿，丰富泛音 | chords, pads, bass |
| `superfm` | FM 合成，金属感 | stabs, 铃声质感 |
| `noise` | 随机噪声 | 纹理, hi-hat |

#### 音色映射规则
| 频带能量特征 | 合成器选择 | 关键参数 |
|-------------|-----------|---------|
| sub_bass 主导 | sine / supersaw | lpf 100-200 |
| bass 主导 + 中高频能量 | supersaw + saturation | lpf 200-600, drive 2-3 |
| presence/bril 有能量 | 必须用 supersaw 或 saturation 生成泛音 | — |
| 有共振峰值 | resonance > 0.5 | — |
| centroid 随时间变化大 | lfo_rate > 0.1 | — |
| 长衰减尾音 | delay feedback > 0.5, reverb mix > 0.2 | — |

### 4. 相似度计算（v2 扩展到 10 指标）

#### 方法：合成对比法

不估算，实际合成再对比。

#### 相似度指标（10 项加权）

**频域（32%）**
| 指标 | 权重 | 说明 |
|------|------|------|
| Band Cosine | 10% | 11 频段能量余弦相似度 |
| Band Correlation | 8% | dB 向量 Pearson 相关 |
| MFCC | 7% | 倒谱系数距离 |
| Centroid | 7% | 频谱重心相似度 |

**时域（48%）**
| 指标 | 权重 | 说明 |
|------|------|------|
| Onset Pattern | 15% | 起音间隔分布 + 密度 + 强度包络 |
| Temporal Envelope | 10% | 时间能量分布形状 |
| Beat Structure | 15% | 逐拍能量 + 逐拍质心 + 动态对比度 |
| RMS Correlation | 8% | RMS 包络相关 |

**谐波/延迟（20%）**
| 指标 | 权重 | 说明 |
|------|------|------|
| Harmonic | 10% | 饱和度 + 基频 + 泛音数量 + 高频相关 |
| Delay Tail | 10% | 衰减包络相关 + 衰减速率相似度 |

#### 迭代优化
如果某指标 < 30%：
- Harmonic 低 → 调整 saturate drive 或换 synth
- Delay Tail 低 → 调整 delay feedback/mix
- Onset Pattern 低 → 调整 degrade/interval/speed
- Band 差异 > 15dB → 调整对应轨道的 lpf/gain

### 5. 输出文件

```
<out_dir>/
├── spectral_data/            # 分析数据
│   ├── summary.json          # 汇总（含 v2 字段）
│   ├── spectral_analysis.png # 7 面板可视化
│   └── *.csv                 # 详细数据
├── tidal_params.json         # 生成的参数
├── tidal_synthesis.wav       # Python 合成对比音频
└── similarity.json           # 10 维相似度结果
```

## 常见问题

### Q: Dub techno 为什么高频差距大？

A: Dub techno 的核心音色来自 **和弦 → 共振 LPF → 磁带饱和 → 延迟** 信号链。饱和会产生高次谐波。如果没有 saturate 模块，合成器的高频能量会严重不足（差 20-30 dB）。

### Q: 和弦和单音的区别？

A: v2 支持 `notes: ["g2", "a#2", "d3"]` 多音合成。多个音符叠加会产生丰富的频谱内容和拍频，比单音更有层次感。dub techno 的 filtered chords 必须用和弦模式。

### Q: Delay 的 lpf 参数是什么？

A: 经典磁带延迟的特征：**每次回声比上一次更暗**。`delay.lpf` 控制每次重复时的低通截止频率，且每次重复会自动衰减（×0.85）。feedback 越高、lpf 越低 = 越 "dub"。

### Q: 相似度多少算合格？

A: ≥20% 合格（形态匹配），≥50% 良好，≥70% 优秀。由于合成器模型和真实音源有本质差异，>80% 极难达到。

## 注意事项

- 网络受限时无法下载外部音频，需用户提供文件
- v1 格式（orbits）仍然支持，自动检测
- 每次迭代调整不宜过大：gain ±0.1, lpf ±200 Hz, drive ±0.5
- saturation drive > 4.0 可能产生过多失真
