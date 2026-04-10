// Game Engine for Bugs-Love
// Bug类 + 碰撞 + 感情线 + 体液系统

class Bug {
  constructor(id, y, isPlayer) {
    this.id = id;
    this.isPlayer = isPlayer;
    this.y = y;
    this.segments = [];
    this.bodyFluid = 100;
    this.deflated = false;
    this.width = 0.6;
    this.connectionLine = null;
    this.alive = true;
    this.mutations = [];
  }

  updateFromSpectrum(freqData, canvasWidth) {
    const segCount = 64;
    const step = Math.floor(freqData.length / segCount);
    this.segments = [];
    for (let i = 0; i < segCount; i++) {
      const val = freqData[i * step] / 255;
      const x = (i / segCount) * canvasWidth;
      const smoothness = 1.0 - (val > 0.7 ? (val - 0.7) / 0.3 : 0);
      this.segments.push({ x, amplitude: val, smoothness, hasConnection: false });
    }
    for (const m of this.mutations) {
      const idx = Math.floor((m.x / canvasWidth) * segCount);
      if (idx >= 0 && idx < segCount) {
        this.segments[idx].smoothness = Math.max(0, this.segments[idx].smoothness - 0.5);
        this.segments[idx].amplitude += m.size;
      }
    }
  }

  deflate(dt) {
    this.deflated = true;
    this.width = Math.max(0.2, this.width - dt * 1.5);
    const drain = dt * 8;
    this.bodyFluid -= drain;
    if (this.bodyFluid <= 0) {
      this.bodyFluid = 0;
      this.alive = false;
    }
  }

  inflate(dt) {
    this.deflated = false;
    this.width = Math.min(1.0, this.width + dt * 1.0);
    const recover = dt * 8 * 0.7;
    this.bodyFluid = Math.min(100, this.bodyFluid + recover);
  }
}

class ConnectionLine {
  constructor(bugA, bugB) {
    this.bugA = bugA;
    this.bugB = bugB;
    this.strength = 0;
    this.points = [];
    this.broken = false;
  }

  update(harmonyRatio, dt) {
    if (this.broken) return;
    this.strength = Math.min(1, this.strength + harmonyRatio * dt * 0.3);
    this.strength = Math.max(0, this.strength - (1 - harmonyRatio) * dt * 0.1);
  }

  break() {
    this.broken = true;
    const loss = this.strength * 30;
    this.bugA.bodyFluid -= loss;
    this.bugB.bodyFluid -= loss;
    if (this.bugA.bodyFluid <= 0) this.bugA.alive = false;
    if (this.bugB.bodyFluid <= 0) this.bugB.alive = false;
  }

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
    this.state = 'playing';
    this.roughnessMap = new Float32Array(64);
  }

  update(freqA, freqB, harmonyData, dt) {
    this.bugA.updateFromSpectrum(freqA, this.canvasWidth);
    this.bugB.updateFromSpectrum(freqB, this.canvasWidth);

    let smoothCollisions = 0;
    let roughCollisions = 0;
    const collisionPoints = [];

    for (let i = 0; i < Math.min(this.bugA.segments.length, this.bugB.segments.length); i++) {
      const segA = this.bugA.segments[i];
      const segB = this.bugB.segments[i];
      const topA = this.bugA.y - segA.amplitude * 50 * this.bugA.width;
      const bottomB = this.bugB.y + segB.amplitude * 50 * this.bugB.width;
      if (topA > bottomB - 10) {
        const avgSmooth = (segA.smoothness + segB.smoothness) / 2;
        if (avgSmooth > 0.6) {
          smoothCollisions++;
          segA.hasConnection = true;
          segB.hasConnection = true;
          collisionPoints.push({ x: segA.x, yA: topA, yB: bottomB });
        } else {
          roughCollisions++;
          this.roughnessMap[i] = Math.min(1, this.roughnessMap[i] + dt * 0.5);
          if (i > 0) this.roughnessMap[i - 1] = Math.min(1, this.roughnessMap[i - 1] + dt * 0.2);
          if (i < 63) this.roughnessMap[i + 1] = Math.min(1, this.roughnessMap[i + 1] + dt * 0.2);
        }
      }
    }

    if (smoothCollisions > 3 && !this.connection) {
      this.connection = new ConnectionLine(this.bugA, this.bugB);
    }
    if (this.connection) {
      this.connection.points = collisionPoints;
      this.connection.update(harmonyData.harmonic, dt);
      if (roughCollisions > smoothCollisions * 2 && this.connection.strength < 0.3) {
        this.connection.break();
        this.connection = null;
      }
      if (this.connection && this.connection.isComplete()) {
        this.state = 'won';
      }
    }

    if (!this.bugA.alive || !this.bugB.alive) {
      this.state = 'lost';
    }
    const roughRatio = this.roughnessMap.reduce((a, b) => a + b, 0) / 64;
    if (roughRatio > 0.8 && (!this.connection || this.connection.broken)) {
      this.state = 'lost';
    }
  }
}
