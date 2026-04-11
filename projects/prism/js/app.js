/* Prism App v0.4.0 — Action-first, timers, no logging */
const App = {
  currentPage: 'today',
  _timers: {},
  _pitch: { active: false, ctx: null, stream: null, frame: null },

  init() {
    setTimeout(() => {
      document.getElementById('loading-screen').classList.add('hidden');
      document.getElementById('app').classList.remove('hidden');
      UI.renderToday();
      this.setupNav();
    }, 600);
  },

  setupNav() {
    document.querySelectorAll('.sidebar-item[data-page]').forEach(item => {
      item.addEventListener('click', () => this.navigate(item.dataset.page));
    });
    document.getElementById('btn-export')?.addEventListener('click', () => {
      const data = Store.exportAll();
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
      const a = document.createElement('a'); a.href = URL.createObjectURL(blob);
      a.download = `prism-${Store.today()}.json`; a.click();
      UI.toast('已导出', 'success');
    });
  },

  navigate(page) {
    this.currentPage = page;
    document.querySelectorAll('.sidebar-item').forEach(i => i.classList.remove('active'));
    document.querySelector(`.sidebar-item[data-page="${page}"]`)?.classList.add('active');
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
    document.getElementById(`page-${page}`)?.classList.add('active');
    const titles = { today: '今日', voice: '声音训练', skin: '护肤', exercise: '锻炼', community: '社群' };
    document.getElementById('page-title').textContent = titles[page] || page;
    switch(page) {
      case 'today': UI.renderToday(); break;
      case 'voice': UI.renderVoicePage(); break;
      case 'skin': UI.renderSkinPage(); break;
      case 'exercise': UI.renderExercisePage(); break;
      case 'community': UI.renderCommunityPage(); break;
    }
  },

  // ---- Plan card expand/collapse ----
  togglePlan(id) {
    document.getElementById(`detail-${id}`)?.classList.toggle('open');
  },

  // ---- Routine step toggle ----
  toggleStep(type, idx, el) {
    Store.toggleRoutineStep(type, idx);
    const done = Store.getRoutineProgressToday(type).includes(idx);
    el.classList.toggle('done', done);
    el.querySelector('.plan-step-check').textContent = done ? '✓' : '';
    // Re-render progress bar
    if (type === 'morning') UI.renderMorningCard();
    else UI.renderEveningCard();
  },

  // ---- Outfit shuffle ----
  shuffleOutfit() {
    UI.renderOutfitCard();
    UI.toast('换了一套 ✨');
  },

  // ---- EXERCISE TIMER ----
  startExerciseTimer(idx, seconds) {
    const key = `exercise-${idx}`;
    if (this._timers[key]) { clearInterval(this._timers[key]); delete this._timers[key]; }

    const btn = document.getElementById(`exercise-btn-${idx}`);
    const bar = document.getElementById(`exercise-bar-${idx}`);
    const fill = document.getElementById(`exercise-fill-${idx}`);
    if (!btn || !bar || !fill) return;

    bar.classList.remove('hidden');
    let remaining = seconds;
    const total = seconds;
    const fmt = s => `${Math.floor(s/60)}:${(s%60).toString().padStart(2,'0')}`;

    btn.textContent = fmt(remaining);
    btn.classList.add('btn-active');
    fill.style.width = '0%';

    this._timers[key] = setInterval(() => {
      remaining--;
      btn.textContent = fmt(remaining);
      fill.style.width = `${((total - remaining) / total) * 100}%`;
      if (remaining <= 0) {
        clearInterval(this._timers[key]);
        delete this._timers[key];
        btn.textContent = '✅';
        btn.classList.remove('btn-active');
        fill.style.width = '100%';
        fill.style.background = 'var(--teal)';
        UI.toast('完成！换下一个 💪', 'success');
      }
    }, 1000);
  },

  // ---- SKIN TIMER ----
  startSkinTimer(idx, seconds) {
    const realIdx = typeof idx === 'string' ? idx : idx;
    const key = `skin-${realIdx}`;
    if (this._timers[key]) { clearInterval(this._timers[key]); delete this._timers[key]; }

    const btnId = typeof idx === 'string' ? `skin-page-btn-${idx.replace('page-','')}` : `skin-btn-${idx}`;
    const barId = typeof idx === 'string' ? `skin-page-bar-${idx.replace('page-','')}` : `skin-bar-${idx}`;
    const fillId = typeof idx === 'string' ? `skin-page-fill-${idx.replace('page-','')}` : `skin-fill-${idx}`;

    const btn = document.getElementById(btnId);
    const bar = document.getElementById(barId);
    const fill = document.getElementById(fillId);
    if (!btn || !bar || !fill) return;

    bar.classList.remove('hidden');
    let remaining = seconds;
    const total = seconds;

    btn.textContent = `${remaining}s`;
    fill.style.width = '0%';

    this._timers[key] = setInterval(() => {
      remaining--;
      btn.textContent = `${remaining}s`;
      fill.style.width = `${((total - remaining) / total) * 100}%`;
      if (remaining <= 0) {
        clearInterval(this._timers[key]);
        delete this._timers[key];
        btn.textContent = '✅';
        fill.style.width = '100%';
        fill.style.background = 'var(--blue)';
        UI.toast('下一步 →', 'success');
      }
    }, 1000);
  },

  // ---- PITCH MONITOR ----
  async startPitch() {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const ctx = new AudioContext();
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 2048;
      ctx.createMediaStreamSource(stream).connect(analyser);
      this._pitch = { active: true, ctx, stream, frame: null };

      document.querySelector('[onclick="App.startPitch()"]')?.classList.add('hidden');
      document.getElementById('btn-stop-pitch')?.classList.remove('hidden');

      const buf = new Float32Array(analyser.fftSize);
      const detect = () => {
        if (!this._pitch.active) return;
        analyser.getFloatTimeDomainData(buf);
        let rms = 0;
        for (let i = 0; i < buf.length; i++) rms += buf[i] * buf[i];
        rms = Math.sqrt(rms / buf.length);
        if (rms > 0.01) {
          let crossings = 0;
          for (let i = 1; i < buf.length; i++) {
            if ((buf[i] >= 0 && buf[i-1] < 0) || (buf[i] < 0 && buf[i-1] >= 0)) crossings++;
          }
          const freq = crossings * ctx.sampleRate / (2 * buf.length);
          if (freq > 60 && freq < 500) {
            document.getElementById('pitch-value').textContent = `${Math.round(freq)} Hz`;
            const ind = document.getElementById('pitch-indicator');
            if (ind) ind.style.bottom = `${Math.min(100, Math.max(0, (freq - 80) / 220 * 100))}%`;
          }
        }
        this._pitch.frame = requestAnimationFrame(detect);
      };
      detect();
    } catch { UI.toast('无法访问麦克风', 'error'); }
  },

  stopPitch() {
    this._pitch.active = false;
    if (this._pitch.frame) cancelAnimationFrame(this._pitch.frame);
    if (this._pitch.ctx) this._pitch.ctx.close();
    if (this._pitch.stream) this._pitch.stream.getTracks().forEach(t => t.stop());
    this._pitch = { active: false, ctx: null, stream: null, frame: null };
    document.querySelector('[onclick="App.startPitch()"]')?.classList.remove('hidden');
    document.getElementById('btn-stop-pitch')?.classList.add('hidden');
    document.getElementById('pitch-value').textContent = '-- Hz';
  },
};

// Init
if ('serviceWorker' in navigator) navigator.serviceWorker.register('sw.js').catch(() => {});
document.addEventListener('DOMContentLoaded', () => App.init());
