/* Prism App v0.3.0 — Proactive Today-first */
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
    }, 800);
  },

  // ---- Navigation ----
  setupNav() {
    document.querySelectorAll('.sidebar-item[data-page]').forEach(item => {
      item.addEventListener('click', () => this.navigate(item.dataset.page));
    });

    // Export
    document.getElementById('btn-export')?.addEventListener('click', () => {
      const data = Store.exportAll();
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
      const a = document.createElement('a'); a.href = URL.createObjectURL(blob);
      a.download = `prism-${Store.today()}.json`; a.click();
      UI.toast('数据已导出 📦', 'success');
    });
    document.getElementById('btn-import')?.addEventListener('click', () =>
      document.getElementById('hidden-import').click());
    document.getElementById('hidden-import')?.addEventListener('change', e => {
      const f = e.target.files[0]; if (!f) return;
      const r = new FileReader();
      r.onload = ev => {
        try { Store.importAll(JSON.parse(ev.target.result)); UI.toast('已导入', 'success'); setTimeout(() => location.reload(), 800); }
        catch { UI.toast('文件格式错误', 'error'); }
      };
      r.readAsText(f);
    });
  },

  navigate(page) {
    this.currentPage = page;
    document.querySelectorAll('.sidebar-item').forEach(i => i.classList.remove('active'));
    document.querySelector(`.sidebar-item[data-page="${page}"]`)?.classList.add('active');
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
    document.getElementById(`page-${page}`)?.classList.add('active');

    const titles = {
      today: '今日', voice: '声音训练', skin: '护肤管理', makeup: '妆容档案',
      fashion: '穿搭灵感', exercise: '身体锻炼', journal: '身体日记',
      community: '社群花园'
    };
    document.getElementById('page-title').textContent = titles[page] || page;

    // Render sub-page
    switch(page) {
      case 'today': UI.renderToday(); break;
      case 'voice': UI.renderVoicePage(); break;
      case 'skin': UI.renderSkinPage(); break;
      case 'makeup': UI.renderMakeupPage(); break;
      case 'fashion': UI.renderFashionPage(); break;
      case 'exercise': UI.renderExercisePage(); break;
      case 'journal': UI.renderJournalPage(); break;
      case 'community': UI.renderCommunityPage(); break;
    }
  },

  // ---- Plan card expand/collapse ----
  togglePlan(id) {
    const detail = document.getElementById(`detail-${id}`);
    if (detail) detail.classList.toggle('open');
  },

  // ---- Morning routine steps ----
  toggleMorningStep(idx, el) {
    const today = Store.today();
    const completions = Store.getRoutineCompletions();
    if (!completions[today]) completions[today] = {};
    if (!completions[today].morning) completions[today].morning = [];
    const arr = completions[today].morning;
    const pos = arr.indexOf(idx);
    if (pos >= 0) { arr.splice(pos, 1); el.classList.remove('done'); el.querySelector('.plan-step-check').textContent = ''; }
    else { arr.push(idx); el.classList.add('done'); el.querySelector('.plan-step-check').textContent = '✓'; }
    Store.setRoutineCompletion(today, 'morning', arr);
    UI.renderMorningPlan();
  },

  toggleEveningStep(idx, el) {
    const today = Store.today();
    const completions = Store.getRoutineCompletions();
    if (!completions[today]) completions[today] = {};
    if (!completions[today].evening) completions[today].evening = [];
    const arr = completions[today].evening;
    const pos = arr.indexOf(idx);
    if (pos >= 0) { arr.splice(pos, 1); el.classList.remove('done'); el.querySelector('.plan-step-check').textContent = ''; }
    else { arr.push(idx); el.classList.add('done'); el.querySelector('.plan-step-check').textContent = '✓'; }
    Store.setRoutineCompletion(today, 'evening', arr);
    UI.renderEveningPlan();
  },

  // ---- Mood ----
  selectMood(btn) {
    document.querySelectorAll('#detail-plan-wellness .mood-btn').forEach(b => b.classList.remove('selected'));
    btn.classList.add('selected');
    document.getElementById('mood-detail')?.classList.remove('hidden');
  },

  saveMood() {
    const sel = document.querySelector('#detail-plan-wellness .mood-btn.selected');
    if (!sel) return UI.toast('请选择心情', 'error');
    Store.addMood({
      mood: +sel.dataset.mood,
      tags: [...document.querySelectorAll('#mood-tags .tag.selected')].map(t => t.dataset.tag),
      note: document.getElementById('mood-note')?.value || ''
    });
    UI.toast('已记录 💜', 'success');
    UI.renderToday();
  },

  // ---- Skin ----
  selectSkinRating(btn) {
    document.querySelectorAll('#detail-plan-skin .mood-btn').forEach(b => b.classList.remove('selected'));
    btn.classList.add('selected');
  },

  saveSkinLog() {
    const sel = document.querySelector('#detail-plan-skin .mood-btn.selected');
    if (!sel) return UI.toast('请选择皮肤状态', 'error');
    Store.addSkinLog({
      rating: +sel.dataset.rating,
      issues: [],
      note: document.getElementById('skin-note')?.value || ''
    });
    UI.toast('护肤记录已保存 🧴', 'success');
    UI.renderToday();
  },

  // ---- Voice ----
  saveVoiceLog() {
    const duration = document.getElementById('voice-duration')?.value || '';
    const note = document.getElementById('voice-note')?.value || '';
    Store.addVoiceLog({ content: note, tags: [], duration: +duration || 0 });
    UI.toast('训练已记录 🎤', 'success');
    UI.renderToday();
  },

  // ---- Pitch monitor ----
  async startPitchMonitor() {
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const ctx = new AudioContext();
      const analyser = ctx.createAnalyser();
      analyser.fftSize = 2048;
      ctx.createMediaStreamSource(stream).connect(analyser);

      this._pitch = { active: true, ctx, stream, frame: null };
      document.getElementById('btn-start-pitch').classList.add('hidden');
      document.getElementById('btn-stop-pitch').classList.remove('hidden');

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
    } catch {
      UI.toast('无法访问麦克风', 'error');
    }
  },

  stopPitchMonitor() {
    this._pitch.active = false;
    if (this._pitch.frame) cancelAnimationFrame(this._pitch.frame);
    if (this._pitch.ctx) this._pitch.ctx.close();
    if (this._pitch.stream) this._pitch.stream.getTracks().forEach(t => t.stop());
    this._pitch = { active: false, ctx: null, stream: null, frame: null };
    document.getElementById('btn-start-pitch')?.classList.remove('hidden');
    document.getElementById('btn-stop-pitch')?.classList.add('hidden');
    document.getElementById('pitch-value').textContent = '-- Hz';
  },

  // ---- Workout ----
  saveWorkoutLog(workoutKey) {
    const duration = document.getElementById('workout-duration')?.value || '';
    const note = document.getElementById('workout-note')?.value || '';
    const workout = PrismData.workouts[workoutKey];
    Store.addWorkoutLog({
      content: `${workout.name}${note ? ' · ' + note : ''}`,
      duration: +duration || workout.duration,
      tags: [workout.name]
    });
    UI.toast('训练已记录 💪', 'success');
    UI.renderToday();
  },

  // ---- Stories ----
  likeStory(id, btn) {
    if (Store.likeStory(id)) {
      btn.classList.add('liked');
      const count = parseInt(btn.textContent.match(/\d+/)?.[0] || 0);
      btn.innerHTML = `❤️ ${count + 1}`;
    }
  },

  // ---- Add routine step (sub-page) ----
  addRoutineStep(type) {
    const input = document.getElementById(`add-${type}-step`);
    const text = input?.value.trim();
    if (!text) return;
    const steps = Store.getRoutine(type);
    steps.push({ text, done: false });
    Store.setRoutine(type, steps);
    input.value = '';
    UI.renderSkinPage();
    UI.toast('已添加', 'success');
  },

  // ---- Add product ----
  addProduct() {
    const name = document.getElementById('product-name')?.value.trim();
    if (!name) return;
    Store.addProduct({ name, type: document.getElementById('product-type')?.value || 'other' });
    document.getElementById('product-name').value = '';
    UI.renderProducts();
    UI.toast('产品已添加', 'success');
  },

  // ---- Add makeup log ----
  saveMakeupLog() {
    Store.addMakeupLog({
      style: document.getElementById('makeup-style')?.value || '',
      techniques: [...document.querySelectorAll('#makeup-techniques .tag.selected')].map(t => t.dataset.tag),
      satisfaction: 0,
      note: document.getElementById('makeup-note')?.value || ''
    });
    UI.toast('妆容已记录 💄', 'success');
    UI.renderMakeupPage();
  },

  // ---- Add outfit log ----
  saveOutfitLog() {
    Store.addOutfitLog({
      occasion: document.getElementById('outfit-occasion')?.value || '',
      desc: document.getElementById('outfit-desc')?.value || '',
      styles: [...document.querySelectorAll('#outfit-styles .tag.selected')].map(t => t.dataset.tag),
      confidence: 0,
      note: ''
    });
    UI.toast('穿搭已记录 👗', 'success');
    UI.renderFashionPage();
  },

  // ---- Body journal ----
  saveBodyJournal() {
    const content = document.getElementById('body-journal-content')?.value.trim();
    if (!content) return UI.toast('写点什么吧', 'error');
    Store.addBodyJournal({
      content,
      feeling: document.querySelector('#body-feeling .tag.selected')?.dataset.tag || '',
      areas: [...document.querySelectorAll('#body-areas .tag.selected')].map(t => t.dataset.tag),
      date: Store.today()
    });
    UI.toast('身体日记已保存 📓', 'success');
    document.getElementById('body-journal-content').value = '';
    UI.renderJournalPage();
  },

  // ---- Share story ----
  shareStory() {
    const content = document.getElementById('story-content')?.value.trim();
    if (!content) return UI.toast('写点什么吧', 'error');
    Store.addStory({
      category: document.getElementById('story-category')?.value || 'general',
      content,
      anon: true
    });
    UI.toast('已匿名分享 🌻', 'success');
    document.getElementById('story-content').value = '';
    UI.renderCommunityPage();
  }
};

// SW registration
if ('serviceWorker' in navigator) navigator.serviceWorker.register('sw.js').catch(() => {});

// Init
document.addEventListener('DOMContentLoaded', () => App.init());
