# 爱情虫 — 原型实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现一个浏览器内可交互的爱情虫原型，两只程序生成旋律的虫通过频率筛检产生碰撞与感情线。

**Architecture:** 三个模块各司其职——音频引擎负责合成与分析，游戏逻辑负责碰撞/感情线/体液，渲染引擎负责 Canvas 绘制。主循环在 `main.js` 中串联。

**Tech Stack:** 纯 HTML5 Canvas 2D + Web Audio API，零依赖，无构建工具。

---

## 文件结构

```
projects/bugs-love/
├── index.html          # 入口，含基础样式
├── audio-engine.js     # Web Audio 合成、滤波、分析
├── game-engine.js      # 虫、碰撞、感情线、体液逻辑
├── renderer.js         # Canvas 绘制（虫、感情线、UI）
└── main.js             # 主循环、输入处理、状态管理
```

## 文件职责

| 文件 | 职责 | 依赖 |
|------|------|------|
| `index.html` | 页面结构、Canvas 元素、脚本加载 | 无 |
| `audio-engine.js` | 创建/控制 OscillatorNode、BiquadFilterNode、AnalyserNode；提供频谱数据 | 无 |
| `game-engine.js` | Bug 类、碰撞检测（光滑/粗糙）、感情线管理、体液经济 | 无 |
| `renderer.js` | 绘制虫曲线、皮肤质感、感情线、体液条、UI 提示 | game-engine.js 数据 |
| `main.js` | requestAnimationFrame 主循环、键盘/鼠标输入、状态机 | 全部 |

---

### Task 1: 项目骨架 + 音频引擎基础

**Files:**
- Create: `projects/bugs-love/index.html`
- Create: `projects/bugs-love/audio-engine.js`

- [ ] **Step 1: 创建 index.html 骨架**

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>爱情虫</title>
<style>
* { margin: 0; padding: 0; box-sizing: border-box; }
body { background: #1a1a2e; overflow: hidden; display: flex; justify-content: center; align-items: center; height: 100vh; }
canvas { display: block; }
#start-btn {
  position: absolute; top: 50%; left: 50%; transform: translate(-50%, -50%);
  padding: 16px 48px; font-size: 24px; background: #e94560; color: white;
  border: none; border-radius: 8px; cursor: pointer; font-family: sans-serif;
  z-index: 10;
}
#start-btn:hover { background: #c73e54; }
#instructions {
  position: absolute; bottom: 20px; left: 50%; transform: translateX(-50%);
  color: #aaa; font-size: 14px; font-family: sans-serif; z-index: 10;
  text-align: center;
}
.hidden { display: none !important; }
</style>
</head>
<body>
<button id="start-btn">开始</button>
<div id="instructions" class="hidden">按住 空格 瘪 → 松开 胀 | 鼠标左右移动控制位置</div>
<canvas id="game"></canvas>
<script src="audio-engine.js"></script>
<script src="game-engine.js"></script>
<script src="renderer.js"></script>
<script src="main.js"></script>
</body>
</html>
```

- [ ] **Step 2: 创建 audio-engine.js — AudioContext 初始化 + 双振荡器**

```javascript
// audio-engine.js — Web Audio 合成引擎
class AudioEngine {
  constructor() {
    this.ctx = null; // AudioContext (用户手势后创建)
    this.bugA = null; // { osc, gain, filter, analyser }
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

    // Bug A — 低频声部 (玩家)
    this.bugA = this._createVoice('sawtooth', 110, 300);
    // Bug B — 中高频声部 (AI)
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

  // 玩家操作：设置滤波频率（瘪=低，胀=高）
  setPlayerFilter(freq) {
    if (!this.bugA) return;
    this.bugA.filter.frequency.setTargetAtTime(freq, this.ctx.currentTime, 0.05);
  }

  // AI 操作
  setAIFilter(freq) {
    if (!this.bugB) return;
    this.bugB.filter.frequency.setTargetAtTime(freq, this.ctx.currentTime, 0.05);
  }

  // 获取频谱数据
  getFrequencyData(voice) {
    const data = new Uint8Array(voice.analyser.frequencyBinCount);
    voice.analyser.getByteFrequencyData(data);
    return data;
  }

  // 获取当前和弦内的音符（简化版：固定和弦进行）
  getCurrentChord(time) {
    const chords = [
      [261.63, 329.63, 392.00], // C major
      [220.00, 277.18, 329.63], // A minor
      [246.94, 311.13, 369.99], // B minor
      [196.00, 246.94, 293.66], // G major
    ];
    const idx = Math.floor(time / (this.beatInterval * 4)) % chords.length;
    return chords[idx];
  }

  // 节拍检测：当前是否在节拍点上
  isOnBeat(time) {
    const phase = (time % this.beatInterval) / this.beatInterval;
    return phase < 0.05 || phase > 0.95;
  }

  // 和谐度检测：两个频率是否和谐
  isHarmonic(freqA, freqB) {
    const ratio = Math.max(freqA, freqB) / Math.min(freqA, freqB);
    const intervals = [1, 1.5, 1.25, 1.333, 2, 2.5, 3]; // unison, fifth, major3, fourth, octave
    return intervals.some(i => Math.abs(ratio - i) < 0.05);
  }

  // 计算两个频谱的冲突/和谐面积比
  computeHarmony(dataA, dataB) {
    let harmonic = 0, conflict = 0;
    const len = Math.min(dataA.length, dataB.length);
    for (let i = 0; i < len; i++) {
      const a = dataA[i] / 255;
      const b = dataB[i] / 255;
      if (a > 0.1 && b > 0.1) {
        // 两个声部在同一频段都有能量
        const binFreq = (i / len) * (this.ctx.sampleRate / 2);
        // 简化：低频重叠 = 冲突（两虫粗糙碰撞），互补 = 和谐
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

  // 随时间变化旋律（在和弦内选音）
  updateMelody(time) {
    const chord = this.getCurrentChord(time);
    // Bug A: 在和弦音之间以节拍为单位切换
    if (this.isOnBeat(time)) {
      const noteA = chord[Math.floor(Math.random() * chord.length)];
      this.bugA.osc.frequency.setTargetAtTime(noteA, this.ctx.currentTime, 0.1);
      const noteB = chord[Math.floor(Math.random() * chord.length)] * 2;
      this.bugB.osc.frequency.setTargetAtTime(noteB, this.ctx.currentTime, 0.1);
    }
  }
}
```

- [ ] **Step 3: 在浏览器中打开 index.html 验证音频能发声**

打开 `index.html`，点击"开始"按钮，确认 AudioContext 初始化并能听到两段不同音色的声音。

- [ ] **Step 4: Commit**

```bash
cd ~/.openclaw/workspace && git add projects/bugs-love/
git commit -m "feat: bugs-love project skeleton + audio engine"
```

---

### Task 2: 游戏引擎 — Bug 类 + 碰撞 + 感情线 + 体液

**Files:**
- Create: `projects/bugs-love/game-engine.js`

- [ ] **Step 1: 创建 Bug 类**

```javascript
// game-engine.js — 游戏逻辑引擎
class Bug {
  constructor(id, y, isPlayer) {
    this.id = id;
    this.isPlayer = isPlayer;
    this.y = y; // 垂直位置（固定）
    this.segments = []; // [{x, smoothness, hasConnection}]
    this.bodyFluid = 100; // 体液 (0-100)
    this.deflated = false;
    this.width = 0.6; // 当前宽度比例 (0.2=瘪, 1.0=胀)
    this.connectionLine = null; // {targetBug, strength, segmentIndices}
    this.alive = true;
    this.mutations = []; // [{x, size}] 瘤状物
  }

  // 从频谱数据生成虫的皮肤段
  updateFromSpectrum(freqData, canvasWidth) {
    const segCount = 64;
    const step = Math.floor(freqData.length / segCount);
    this.segments = [];
    for (let i = 0; i < segCount; i++) {
      const val = freqData[i * step] / 255;
      const x = (i / segCount) * canvasWidth;
      // 光滑度：高频能量低 = 光滑，高频能量高 = 粗糙（噪音多）
      const smoothness = 1.0 - (val > 0.7 ? (val - 0.7) / 0.3 : 0);
      this.segments.push({ x, amplitude: val, smoothness, hasConnection: false });
    }
    // 叠加变异瘤
    for (const m of this.mutations) {
      const idx = Math.floor((m.x / canvasWidth) * segCount);
      if (idx >= 0 && idx < segCount) {
        this.segments[idx].smoothness = Math.max(0, this.segments[idx].smoothness - 0.5);
        this.segments[idx].amplitude += m.size;
      }
    }
  }

  // 瘪操作
  deflate(dt) {
    this.deflated = true;
    this.width = Math.max(0.2, this.width - dt * 1.5);
    const drain = dt * 8; // 每秒消耗 8 体液
    this.bodyFluid -= drain;
    if (this.bodyFluid <= 0) {
      this.bodyFluid = 0;
      this.alive = false;
    }
  }

  // 胀操作（部分回收）
  inflate(dt) {
    this.deflated = false;
    const oldWidth = this.width;
    this.width = Math.min(1.0, this.width + dt * 1.0);
    // 回收 70% 的体液
    const recover = dt * 8 * 0.7;
    this.bodyFluid = Math.min(100, this.bodyFluid + recover);
  }
}

class ConnectionLine {
  constructor(bugA, bugB) {
    this.bugA = bugA;
    this.bugB = bugB;
    this.strength = 0; // 0-1
    this.points = []; // [{x, yA, yB}] 连接点
    this.broken = false;
  }

  // 更新感情线
  update(harmonyRatio, dt) {
    if (this.broken) return;
    // 和谐时强化
    this.strength = Math.min(1, this.strength + harmonyRatio * dt * 0.3);
    // 冲突时弱化
    this.strength = Math.max(0, this.strength - (1 - harmonyRatio) * dt * 0.1);
  }

  // 断开感情线
  break() {
    this.broken = true;
    // 断开时的体液损失 = 强度 * 30
    const loss = this.strength * 30;
    this.bugA.bodyFluid -= loss;
    this.bugB.bodyFluid -= loss;
    if (this.bugA.bodyFluid <= 0) this.bugA.alive = false;
    if (this.bugB.bodyFluid <= 0) this.bugB.alive = false;
  }

  // 判断是否胜利
  isComplete() {
    return this.strength >= 0.95 && !this.broken;
  }
}

class GameEngine {
  constructor(canvasWidth, canvasHeight) {
    this.canvasWidth = canvasWidth;
    this.canvasHeight = canvasHeight;
    this.bugA = new Bug('A', canvasHeight * 0.35, true);
    this.bugB = new Bug('B', canvasHeight * 0.65, false);
    this.connection = null;
    this.state = 'playing'; // 'playing' | 'won' | 'lost'
    this.roughnessMap = new Float32Array(64); // 粗糙蔓延追踪
  }

  // 碰撞检测 + 感情线管理
  update(freqA, freqB, harmonyData, dt) {
    this.bugA.updateFromSpectrum(freqA, this.canvasWidth);
    this.bugB.updateFromSpectrum(freqB, this.canvasWidth);

    // 检测碰撞
    let smoothCollisions = 0;
    let roughCollisions = 0;
    const collisionPoints = [];

    for (let i = 0; i < Math.min(this.bugA.segments.length, this.bugB.segments.length); i++) {
      const segA = this.bugA.segments[i];
      const segB = this.bugB.segments[i];
      // 两虫的振幅是否重叠
      const topA = this.bugA.y - segA.amplitude * 50 * this.bugA.width;
      const bottomB = this.bugB.y + segB.amplitude * 50 * this.bugB.width;
      if (topA > bottomB - 10) { // 碰撞
        const avgSmooth = (segA.smoothness + segB.smoothness) / 2;
        if (avgSmooth > 0.6) {
          smoothCollisions++;
          segA.hasConnection = true;
          segB.hasConnection = true;
          collisionPoints.push({ x: segA.x, yA: topA, yB: bottomB });
        } else {
          roughCollisions++;
          // 粗糙蔓延
          this.roughnessMap[i] = Math.min(1, this.roughnessMap[i] + dt * 0.5);
          if (i > 0) this.roughnessMap[i - 1] = Math.min(1, this.roughnessMap[i - 1] + dt * 0.2);
          if (i < 63) this.roughnessMap[i + 1] = Math.min(1, this.roughnessMap[i + 1] + dt * 0.2);
        }
      }
    }

    // 感情线逻辑
    if (smoothCollisions > 3 && !this.connection) {
      this.connection = new ConnectionLine(this.bugA, this.bugB);
    }
    if (this.connection) {
      this.connection.points = collisionPoints;
      this.connection.update(harmonyData.harmonic, dt);
      // 粗糙碰撞太多 → 断开
      if (roughCollisions > smoothCollisions * 2 && this.connection.strength < 0.3) {
        this.connection.break();
        this.connection = null;
      }
      // 胜利检查
      if (this.connection.isComplete()) {
        this.state = 'won';
      }
    }

    // 失败检查
    if (!this.bugA.alive || !this.bugB.alive) {
      this.state = 'lost';
    }
    // 过度粗糙检查：大部分皮肤已粗糙且无感情线
    const roughRatio = this.roughnessMap.reduce((a, b) => a + b, 0) / 64;
    if (roughRatio > 0.8 && (!this.connection || this.connection.broken)) {
      this.state = 'lost';
    }
  }
}
```

- [ ] **Step 2: 验证 — 在控制台创建 GameEngine 实例，确认 Bug 初始化正确**

在浏览器 console 中运行：
```javascript
const g = new GameEngine(800, 600);
console.log(g.bugA, g.bugB, g.state);
```

- [ ] **Step 3: Commit**

```bash
git add projects/bugs-love/game-engine.js
git commit -m "feat: Bug class, collision detection, connection lines, body fluid"
```

---

### Task 3: 渲染引擎

**Files:**
- Create: `projects/bugs-love/renderer.js`

- [ ] **Step 1: 创建 renderer.js**

```javascript
// renderer.js — Canvas 渲染
class Renderer {
  constructor(canvas) {
    this.canvas = canvas;
    this.ctx = canvas.getContext('2d');
    this.width = canvas.width;
    this.height = canvas.height;
    this.time = 0;
  }

  clear() {
    this.ctx.fillStyle = '#1a1a2e';
    this.ctx.fillRect(0, 0, this.width, this.height);
  }

  // 绘制一只虫
  drawBug(bug, color, glowColor) {
    const ctx = this.ctx;
    if (!bug.segments.length) return;

    ctx.save();

    // 虫身体轮廓
    ctx.beginPath();
    ctx.moveTo(0, bug.y);

    for (let i = 0; i < bug.segments.length; i++) {
      const seg = bug.segments[i];
      const amp = seg.amplitude * 50 * bug.width;
      const roughness = 1 - seg.smoothness;

      // 上半部分
      if (roughness > 0.4) {
        // 粗糙：锯齿
        const jitter = roughness * 8 * Math.sin(i * 7 + this.time * 3);
        ctx.lineTo(seg.x, bug.y - amp + jitter);
      } else {
        // 光滑：贝塞尔
        const nextSeg = bug.segments[Math.min(i + 1, bug.segments.length - 1)];
        const nextAmp = nextSeg.amplitude * 50 * bug.width;
        const cpx = (seg.x + nextSeg.x) / 2;
        ctx.quadraticCurveTo(cpx, bug.y - amp, nextSeg.x, bug.y - nextAmp);
      }
    }

    // 下半部分（镜像）
    for (let i = bug.segments.length - 1; i >= 0; i--) {
      const seg = bug.segments[i];
      const amp = seg.amplitude * 50 * bug.width;
      const roughness = 1 - seg.smoothness;
      if (roughness > 0.4) {
        const jitter = roughness * 8 * Math.cos(i * 5 + this.time * 2);
        ctx.lineTo(seg.x, bug.y + amp + jitter);
      } else {
        const prevSeg = bug.segments[Math.max(i - 1, 0)];
        const prevAmp = prevSeg.amplitude * 50 * bug.width;
        const cpx = (seg.x + prevSeg.x) / 2;
        ctx.quadraticCurveTo(cpx, bug.y + amp, prevSeg.x, bug.y + prevAmp);
      }
    }

    ctx.closePath();

    // 渐变填充
    const grad = ctx.createLinearGradient(0, bug.y - 60, 0, bug.y + 60);
    grad.addColorStop(0, color);
    grad.addColorStop(0.5, glowColor);
    grad.addColorStop(1, color);
    ctx.fillStyle = grad;
    ctx.fill();

    // 光滑段发光
    ctx.shadowColor = glowColor;
    ctx.shadowBlur = 10;
    ctx.strokeStyle = glowColor + '80';
    ctx.lineWidth = 1;
    ctx.stroke();
    ctx.shadowBlur = 0;

    // 频谱填充（体内）
    this.drawSpectrumFill(bug, ctx);

    // 瘤状物
    for (const m of bug.mutations) {
      ctx.beginPath();
      ctx.arc(m.x, bug.y, m.size * 15, 0, Math.PI * 2);
      ctx.fillStyle = '#4a3728';
      ctx.fill();
    }

    ctx.restore();
  }

  drawSpectrumFill(bug, ctx) {
    // 在虫体内绘制频谱条纹
    for (const seg of bug.segments) {
      const amp = seg.amplitude * 50 * bug.width;
      if (amp < 2) continue;
      const hue = seg.amplitude * 240; // 低频=红，高频=蓝
      ctx.fillStyle = `hsla(${hue}, 70%, 50%, 0.15)`;
      ctx.fillRect(seg.x - 2, bug.y - amp * 0.8, 4, amp * 1.6);
    }
  }

  // 绘制感情线
  drawConnection(line) {
    if (!line || line.broken || line.points.length === 0) return;
    const ctx = this.ctx;
    const alpha = line.strength;
    const width = 1 + line.strength * 5;

    ctx.save();
    ctx.strokeStyle = `rgba(255, 100, 150, ${alpha})`;
    ctx.lineWidth = width;
    ctx.shadowColor = '#ff6496';
    ctx.shadowBlur = 8 + line.strength * 12;

    for (const pt of line.points) {
      ctx.beginPath();
      ctx.moveTo(pt.x, pt.yA);
      ctx.lineTo(pt.x, pt.yB);
      ctx.stroke();

      // 养分粒子
      if (line.strength > 0.2) {
        const particleY = pt.yA + (pt.yB - pt.yA) * ((Math.sin(this.time * 3 + pt.x) + 1) / 2);
        ctx.beginPath();
        ctx.arc(pt.x, particleY, 2 + line.strength * 2, 0, Math.PI * 2);
        ctx.fillStyle = `rgba(255, 200, 100, ${alpha})`;
        ctx.fill();
      }
    }
    ctx.restore();
  }

  // 绘制体液条
  drawFluidBar(bug, x, y) {
    const ctx = this.ctx;
    const w = 120, h = 8;
    // 背景
    ctx.fillStyle = '#333';
    ctx.fillRect(x, y, w, h);
    // 体液
    const fluidW = (bug.bodyFluid / 100) * w;
    const fluidColor = bug.bodyFluid > 30 ? '#4fc3f7' : '#e94560';
    ctx.fillStyle = fluidColor;
    ctx.fillRect(x, y, fluidW, h);
    // 文字
    ctx.fillStyle = '#fff';
    ctx.font = '12px sans-serif';
    ctx.fillText(`${bug.isPlayer ? '你' : 'AI'} 体液: ${Math.round(bug.bodyFluid)}%`, x, y - 4);
  }

  // 绘制胜利/失败
  drawEndState(state) {
    const ctx = this.ctx;
    ctx.save();
    ctx.fillStyle = 'rgba(0,0,0,0.6)';
    ctx.fillRect(0, 0, this.width, this.height);
    ctx.textAlign = 'center';
    ctx.font = '48px sans-serif';
    if (state === 'won') {
      ctx.fillStyle = '#ff6496';
      ctx.fillText('新生命诞生', this.width / 2, this.height / 2);
      ctx.font = '20px sans-serif';
      ctx.fillStyle = '#aaa';
      ctx.fillText('两颗心找到了共鸣的频率', this.width / 2, this.height / 2 + 40);
    } else {
      ctx.fillStyle = '#e94560';
      ctx.fillText('枯萎', this.width / 2, this.height / 2);
      ctx.font = '20px sans-serif';
      ctx.fillStyle = '#aaa';
      ctx.fillText('失去了连接的能力', this.width / 2, this.height / 2 + 40);
    }
    ctx.restore();
  }

  // 绘制操作提示
  drawControlsHint() {
    const ctx = this.ctx;
    ctx.save();
    ctx.fillStyle = '#666';
    ctx.font = '13px sans-serif';
    ctx.textAlign = 'center';
    ctx.fillText('按住 空格 = 瘪（掏空自己） | 松开 = 胀（恢复饱满）', this.width / 2, this.height - 12);
    ctx.restore();
  }

  // 主绘制
  render(game) {
    this.time += 0.016;
    this.clear();
    this.drawBug(game.bugA, '#e94560', '#ff8a9e');
    this.drawBug(game.bugB, '#4fc3f7', '#80d8ff');
    this.drawConnection(game.connection);
    this.drawFluidBar(game.bugA, 20, 30);
    this.drawFluidBar(game.bugB, this.width - 140, 30);
    this.drawControlsHint();
    if (game.state !== 'playing') {
      this.drawEndState(game.state);
    }
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add projects/bugs-love/renderer.js
git commit -m "feat: canvas renderer with bug curves, connection lines, fluid bars"
```

---

### Task 4: 主循环串联

**Files:**
- Create: `projects/bugs-love/main.js`

- [ ] **Step 1: 创建 main.js**

```javascript
// main.js — 主循环、输入、状态机
(function () {
  const canvas = document.getElementById('game');
  const startBtn = document.getElementById('start-btn');
  const instructions = document.getElementById('instructions');

  canvas.width = window.innerWidth;
  canvas.height = window.innerHeight;

  let audio = null;
  let game = null;
  let renderer = null;
  let running = false;
  let spaceHeld = false;

  startBtn.addEventListener('click', () => {
    audio = new AudioEngine();
    audio.init();
    game = new GameEngine(canvas.width, canvas.height);
    renderer = new Renderer(canvas);
    startBtn.classList.add('hidden');
    instructions.classList.remove('hidden');
    running = true;
    requestAnimationFrame(loop);
  });

  // 键盘输入
  document.addEventListener('keydown', (e) => {
    if (e.code === 'Space') {
      e.preventDefault();
      spaceHeld = true;
    }
  });
  document.addEventListener('keyup', (e) => {
    if (e.code === 'Space') {
      spaceHeld = false;
    }
  });

  // 重新开始
  document.addEventListener('keydown', (e) => {
    if (e.code === 'KeyR' && game && game.state !== 'playing') {
      game = new GameEngine(canvas.width, canvas.height);
      // 重新初始化音频
      audio.bugA.osc.stop();
      audio.bugB.osc.stop();
      audio.init();
    }
  });

  let lastTime = 0;

  function loop(timestamp) {
    if (!running) return;
    const dt = Math.min((timestamp - lastTime) / 1000, 0.05); // cap dt
    lastTime = timestamp;

    // 玩家操作
    if (spaceHeld) {
      game.bugA.deflate(dt);
      audio.setPlayerFilter(200); // 瘪：只留低频
    } else {
      game.bugA.inflate(dt);
      audio.setPlayerFilter(4000); // 胀：全频
    }

    // AI 自动调整
    const freqDataA = audio.getFrequencyData(audio.bugA);
    const freqDataB = audio.getFrequencyData(audio.bugB);
    const harmony = audio.computeHarmony(freqDataA, freqDataB);

    // AI 逻辑：冲突多时降低自己的滤波
    if (harmony.conflict > 0.3) {
      audio.setAIFilter(800);
    } else {
      audio.setAIFilter(3000);
    }

    // 旋律更新
    const time = audio.ctx.currentTime;
    audio.updateMelody(time);

    // 游戏逻辑更新
    if (game.state === 'playing') {
      game.update(freqDataA, freqDataB, harmony, dt);
    }

    // 渲染
    renderer.render(game);

    requestAnimationFrame(loop);
  }

  // 窗口大小变化
  window.addEventListener('resize', () => {
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    if (renderer) {
      renderer.width = canvas.width;
      renderer.height = canvas.height;
    }
    if (game) {
      game.canvasWidth = canvas.width;
      game.canvasHeight = canvas.height;
      game.bugA.y = canvas.height * 0.35;
      game.bugB.y = canvas.height * 0.65;
    }
  });
})();
```

- [ ] **Step 2: 在浏览器中打开完整原型，测试核心循环**

打开 `projects/bugs-love/index.html`，点击开始：
1. 能听到两段旋律
2. 按住空格能感受到声音变薄（瘪），松开恢复（胀）
3. 能看到两只虫的身体变化
4. 光滑碰撞时出现感情线
5. 体液条在瘪时减少

- [ ] **Step 3: Commit**

```bash
git add projects/bugs-love/main.js
git commit -m "feat: main loop with input handling, AI logic, game state"
```

---

### Task 5: 调优与打磨

**Files:**
- Modify: `projects/bugs-love/audio-engine.js`
- Modify: `projects/bugs-love/game-engine.js`
- Modify: `projects/bugs-love/renderer.js`
- Modify: `projects/bugs-love/main.js`

- [ ] **Step 1: 调整和谐/冲突阈值**

根据实际游戏体验调整以下参数（在 `game-engine.js` 和 `main.js` 中）：
- `smoothCollisions > 3` → 降低到 2（更容易建立感情线）
- `roughCollisions > smoothCollisions * 2` → 调整为 `* 1.5`
- 体液消耗速率 `dt * 8` → 根据实际节奏调整
- 体液回收率 `0.7` → 根据策略感调整

- [ ] **Step 2: 添加音效反馈**

在 `audio-engine.js` 中添加：
- 感情线建立时：播放一个上升的和弦琶音（短促的 C-E-G）
- 感情线断裂时：播放不和谐音（增四度 C-F#）
- 胜利时：播放完整的大三和弦持续音

- [ ] **Step 3: 添加视觉反馈增强**

在 `renderer.js` 中：
- 节拍点：每拍时虫的身体短暂脉冲（放大 5% 再恢复）
- 瘤出现时：添加粒子效果
- 感情线强化时：线的颜色从粉变金

- [ ] **Step 4: Final commit**

```bash
git add -A
git commit -m "feat: bugs-love prototype v1 complete — tune, polish, sound FX"
```

---

## Spec 覆盖自检

| Spec 要求 | 覆盖任务 |
|-----------|----------|
| 两段程序生成旋律同时播放 | Task 1 |
| 节拍可视化（脉冲） | Task 5 |
| 玩家瘪/胀控制频谱 | Task 4 |
| AI 自动调整频谱 | Task 4 |
| 碰撞检测（光滑/粗糙） | Task 2 |
| 感情线生成/强化/断裂 | Task 2 |
| 体液消耗与回收 | Task 2 |
| 成功/失败判定 | Task 2 |

✅ 全部覆盖，无遗漏。
