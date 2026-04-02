# TidalCycles & SuperCollider Live Coding Skill

## 概述

辅助用户进行 TidalCycles + SuperCollider 现场编码（Live Coding）音乐创作。涵盖模式编写、合成器设计、效果处理、实时表演辅助。

## 触发条件

当用户涉及以下内容时使用此 skill：
- TidalCycles / Tidal 模式编写
- SuperCollider 合成器/效果器设计
- Live Coding / Algorave / 即兴编码音乐
- 音序器 pattern 设计
- 声音合成（加法、减法、FM、粒状等）
- Strudel / Hydra 等相关工具

## 核心知识库

### TidalCycles 基础

#### 基本 Pattern
```haskell
-- 单音序列
d1 $ sound "bd sn bd sn"

-- SuperDirt 合成器音色
d1 $ note "c4 e4 g4 c5" # s "superfm"

-- 休止与空拍
d1 $ sound "bd ~ sn ~"

-- 细分（Euclidean rhythm）
d1 $ sound "bd(3,8)"      -- 3 hits in 8 steps
d1 $ sound "bd(5,8,1)"    -- rotation offset

-- 多层 pattern
d1 $ sound "bd(3,8)"
d2 $ sound "sn(5,8)"
d3 $ note "c4*8" # s "supersaw"
```

#### Pattern 变换
```haskell
-- 速度
d1 $ fast 2 $ sound "bd sn"           -- 加倍
d1 $ slow 2 $ sound "bd sn"           -- 减半
d1 $ iter 4 $ sound "bd sn hh cp"     -- 每轮移位

-- 逆行 / 镜像
d1 $ rev $ sound "bd sn hh cp"

-- 随机
d1 $ degradeBy 0.3 $ sound "bd sn"    -- 30% 概率丢弃
d1 $ sometimes (rev) $ sound "bd sn"  -- 有时反转
d1 $ every 4 (fast 2) $ sound "bd sn"

-- 切片
d1 $ chunk 4 (fast 2) $ sound "bd sn hh cp"
d1 $ within (0.25, 0.75) (fast 2) $ sound "bd sn hh cp"

-- 堆叠
d1 $ stack [
  sound "bd(3,8)",
  sound "sn(5,8)" # gain 0.8,
  fast 4 $ sound "hh*2" # gain 0.5
]
```

#### 参数控制
```haskell
-- 音量
d1 $ sound "bd sn" # gain "0.8 1.2"

-- 滤波器
d1 $ sound "bd sn" # lpf 1000 # resonance 0.4
d1 $ sound "bd sn" # hpf (range 200 4000 $ slow 4 sine)

-- 延迟
d1 $ sound "bd sn" # delay 0.5 # delaytime (1/3) # delayfeedback 0.6

-- 混响
d1 $ sound "bd sn" # room 0.3 # sz 0.7

-- 音高
d1 $ note "0 4 7 12" # s "superfm" # octave 4
```

### SuperCollider 常用合成器

#### 加法合成
```supercollider
(
SynthDef(\additive, {
  |out=0, freq=440, amp=0.3, gate=1|
  var sig = Mix.ar(
    Array.fill(8, { |i|
      SinOsc.ar(freq * (i + 1)) / (i + 1)
    })
  );
  sig = sig * EnvGen.kr(Env.asr(0.01, 1, 0.3), gate, doneAction: 2);
  Out.ar(out, sig ! 2 * amp);
}).add;
)
```

#### FM 合成
```supercollider
(
SynthDef(\fm, {
  |out=0, freq=440, modRatio=2, modIndex=3, amp=0.3, gate=1|
  var mod = SinOsc.ar(freq * modRatio) * freq * modIndex;
  var sig = SinOsc.ar(freq + mod);
  sig = sig * EnvGen.kr(Env.adsr(0.01, 0.1, 0.7, 0.5), gate, doneAction: 2);
  Out.ar(out, sig ! 2 * amp);
}).add;
)
```

#### 粒状合成
```supercollider
(
SynthDef(\grain, {
  |out=0, buf, rate=1, amp=0.5|
  var sig = GrainBuf.ar(
    2, Impulse.ar(20), 0.1,
    buf, rate,
    LFNoise1.kr(0.5).range(0, 1),
    4, -1
  );
  Out.ar(out, sig * amp);
}).add;
)
```

### 常用技巧

#### Sidechain 压缩效果
```haskell
d1 $ sound "bd(3,8)"
d2 $ sound "cp(5,8)" # squiz 2
-- 利用 gain pattern 模拟 pumping
d2 $ fast 8 $ gain "~ 1 ~ 1 ~ 1 ~ 1" # sound "hh"
```

#### 渐变 morph
```haskell
-- lerp between patterns
d1 $ interpolateFromTo 8 "bd sn" "cp hh"

-- 构建 tension/release
d1 $ slow 8 $ stack [
  every 4 (rev) $ sound "bd(3,8)",
  degradeBy (slow 8 $ range 0 0.5 $ sine) $ sound "hh*8"
]
```

#### 时间切片
```haskell
-- 把一段 pattern 拆成小片再重组
d1 $ chop 8 $ sound "bd sn hh cp"
d1 $ striate 16 $ sound "supersaw" # note "c4"
```

### Strudel（浏览器版 Tidal）
```javascript
// Strudel 基本用法
s("bd sn bd sn")
.note("c e g c5").s("sawtooth")
.fast(2)
.lpf(sine.range(300, 3000).slow(4))
.delay(0.5)
```

## 工作流程

### 1. 理解用户意图
- 用户想要什么风格？（techno、ambient、drill & bass、noise...）
- 是即兴创作还是有具体参考？
- 需要从零开始还是改进已有代码？

### 2. 生成代码
- 根据风格选择合适的音色、节奏型、效果链
- TidalCycles 代码可直接在 Estuary / Tidal REPL 中运行
- SuperCollider SynthDef 需要先加载到 SC 服务器

### 3. 解释与教学
- 解释每个 pattern 的节奏逻辑
- 说明参数的作用和推荐范围
- 提供变奏建议

### 4. 调试辅助
- 常见错误排查（语法、采样路径、SynthDef 未加载等）
- 性能优化（减少 CPU 开销的技巧）

## 风格速查

| 风格 | BPM | 常用 pattern | 特征 |
|------|-----|-------------|------|
| Techno | 120-140 | `bd(3,8)`, 16th hi-hats | 工业感、重复 |
| House | 120-130 | 四拍底鼓 + off-beat hh | 律动感 |
| Jungle/DnB | 160-180 | `fast 2`, breakbeats | 快速 break |
| Ambient | 60-90 | `slow`, reverb heavy | 空间感 |
| Glitch | varies | `chop`, `striate`, `degrade` | 碎片化 |
| Minimal | 120-135 | 极简 pattern + 微变化 | 细节变化 |

## 常见问题

- **Tidal 没声音？** → 确认 SuperDirt 在 SuperCollider 中已启动 (`SuperDirt.start`)
- **延迟不生效？** → 检查 `delayfeedback` 是否为 0
- **Pattern 不变化？** → 确认 `cps` 设置正确 (`setcps (120/60/4)`)
- **CPU 过高？** → 减少 Polyphony、降低 grain rate
