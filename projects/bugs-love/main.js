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
      if (audio.bugA && audio.bugB) {
        try { audio.bugA.osc.stop(); } catch (e) {}
        try { audio.bugB.osc.stop(); } catch (e) {}
      }
      audio.init();
      game = new GameEngine(canvas.width, canvas.height);
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

    // 获取频谱数据
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
    if (audio.ctx) {
      audio.updateMelody(audio.ctx.currentTime);
    }

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
