/* Prism App - Main logic & event handlers */
const Prism = {
  currentPage: 'dashboard',
  _timers: {},

  init() {
    setTimeout(() => {
      document.getElementById('loading-screen').classList.add('hidden');
      document.getElementById('app').classList.remove('hidden');
      this.setupNav();
      this.setupEvents();
      this.renderAll();
      UI.renderGreeting();
    }, 1000);
  },

  setupNav() {
    document.querySelectorAll('.nav-item').forEach(item => {
      item.addEventListener('click', () => this.navigate(item.dataset.page));
    });
    document.getElementById('sidebar-toggle')?.addEventListener('click', () =>
      document.getElementById('sidebar').classList.toggle('collapsed'));
    document.getElementById('sidebar-toggle-mobile')?.addEventListener('click', () =>
      document.getElementById('sidebar').classList.toggle('mobile-open'));

    // Export/Import
    document.getElementById('btn-export')?.addEventListener('click', () => {
      const data = Store.exportAll();
      const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
      const a = document.createElement('a'); a.href = URL.createObjectURL(blob);
      a.download = `prism-${new Date().toISOString().split('T')[0]}.json`; a.click();
      UI.toast('数据已导出 📦', 'success');
    });
    document.getElementById('btn-import')?.addEventListener('click', () =>
      document.getElementById('hidden-import').click());
    document.getElementById('hidden-import')?.addEventListener('change', e => {
      const f = e.target.files[0]; if (!f) return;
      const r = new FileReader();
      r.onload = ev => { try { Store.importAll(JSON.parse(ev.target.result)); UI.toast('已导入', 'success'); setTimeout(()=>location.reload(),1000); } catch { UI.toast('文件格式错误','error'); } };
      r.readAsText(f);
    });
  },

  navigate(page) {
    this.currentPage = page;
    document.querySelectorAll('.nav-item').forEach(i => i.classList.remove('active'));
    document.querySelector(`.nav-item[data-page="${page}"]`)?.classList.add('active');
    document.querySelectorAll('.page').forEach(p => p.classList.remove('active'));
    document.getElementById(`page-${page}`)?.classList.add('active');
    const titles = { dashboard:'今日概览', voice:'声音训练', skin:'护肤管理', makeup:'妆容档案',
      fashion:'穿搭灵感', posture:'形体仪态', exercise:'身体锻炼', routine:'每日流程',
      'body-journal':'身体日记', community:'社群花园', resources:'资源库', wellness:'情绪支持' };
    document.getElementById('page-title').textContent = titles[page] || page;
    document.getElementById('sidebar').classList.remove('mobile-open');
    this.renderPage(page);
  },

  renderPage(page) {
    switch(page) {
      case 'dashboard': this.renderDashboard(); break;
      case 'voice': UI.renderVoiceLogs(); break;
      case 'skin': UI.renderSkinRoutine('morning'); UI.renderProducts(); break;
      case 'makeup': UI.renderMakeupHistory(); break;
      case 'fashion': UI.renderOutfitHistory(); break;
      case 'posture': UI.renderPostureLogs(); break;
      case 'exercise': UI.renderWorkoutPlan('full'); UI.renderWorkoutHistory(); break;
      case 'routine': UI.renderRoutine('morning'); UI.renderRoutine('evening'); UI.renderRoutineStats(); break;
      case 'body-journal': UI.renderBodyJournals(); break;
      case 'community': UI.renderStories(); break;
      case 'wellness': UI.renderAffirmation(); UI.renderSelfCareCheck(); break;
    }
  },

  renderAll() {
    this.renderDashboard();
    UI.renderAffirmation();
    UI.renderSelfCareCheck();
  },

  renderDashboard() {
    UI.renderStreak();
    UI.renderDashboardRoutine();
    UI.renderDashboardTraining();
    UI.renderDashboardSkin();
    UI.renderDashboardOutfit();
  },

  // ---- Timer utility ----
  startTimer(elId, seconds) {
    if (this._timers[elId]) { clearInterval(this._timers[elId]); this._timers[elId] = null; }
    let remaining = seconds;
    const el = document.getElementById(elId);
    const fmt = s => `${Math.floor(s/60)}:${(s%60).toString().padStart(2,'0')}`;
    el.textContent = fmt(remaining);
    this._timers[elId] = setInterval(() => {
      remaining--;
      el.textContent = fmt(remaining);
      if (remaining <= 0) { clearInterval(this._timers[elId]); el.textContent = '✅ 完成！'; UI.toast('计时结束！', 'success'); }
    }, 1000);
  },

  // ---- Routine toggle ----
  toggleRoutineStep(type, idx, el) {
    const today = new Date().toISOString().split('T')[0];
    const completions = Store.getRoutineCompletions();
    if (!completions[today]) completions[today] = {};
    if (!completions[today][type]) completions[today][type] = [];
    const arr = completions[today][type];
    const pos = arr.indexOf(idx);
    if (pos >= 0) { arr.splice(pos, 1); el.classList.remove('done'); }
    else { arr.push(idx); el.classList.add('done'); el.classList.add('completing'); }
    Store.setRoutineCompletion(today, type, arr);
  },

  // ---- Self-care toggle ----
  toggleSelfCare(el, text) {
    const done = Store.toggleSelfCareItem(text);
    el.classList.toggle('done', done);
    if (done) el.classList.add('completing');
  },

  // ---- Story like ----
  likeStory(id, btn) {
    if (Store.likeStory(id)) {
      btn.classList.add('liked');
      const count = parseInt(btn.textContent.match(/\d+/)?.[0] || 0);
      btn.innerHTML = `❤️ ${count + 1}`;
    }
  },

  // ---- Tag toggling helper ----
  setupTagToggles(containerId, single=false) {
    document.querySelectorAll(`#${containerId} .tag`).forEach(tag => {
      tag.addEventListener('click', () => {
        if (single) document.querySelectorAll(`#${containerId} .tag`).forEach(t => t.classList.remove('selected'));
        tag.classList.toggle('selected');
      });
    });
  },

  // ---- Setup all events ----
  setupEvents() {
    // Mood (wellness page)
    document.querySelectorAll('#mood-scale .mood-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('#mood-scale .mood-btn').forEach(b => b.classList.remove('selected'));
        btn.classList.add('selected');
        document.getElementById('mood-detail').classList.remove('hidden');
      });
    });
    this.setupTagToggles('mood-tags');
    document.getElementById('btn-save-mood')?.addEventListener('click', () => {
      const sel = document.querySelector('#mood-scale .mood-btn.selected');
      if (!sel) return UI.toast('请选择心情','error');
      Store.addMood({ mood: +sel.dataset.mood, tags: [...document.querySelectorAll('#mood-tags .tag.selected')].map(t=>t.dataset.tag), note: document.getElementById('mood-note').value });
      UI.toast('已记录 💜','success');
      document.querySelectorAll('#mood-scale .mood-btn').forEach(b=>b.classList.remove('selected'));
      document.querySelectorAll('#mood-tags .tag').forEach(t=>t.classList.remove('selected'));
      document.getElementById('mood-note').value='';
      document.getElementById('mood-detail').classList.add('hidden');
    });

    // Breathing
    let breathingInterval = null, isBreathing = false;
    document.getElementById('btn-start-breathing')?.addEventListener('click', () => {
      const btn = document.getElementById('btn-start-breathing');
      const circle = document.getElementById('breath-circle');
      const text = document.getElementById('breath-text');
      if (isBreathing) {
        isBreathing = false; clearTimeout(breathingInterval);
        circle.className='breath-circle'; text.textContent='准备开始'; btn.textContent='开始练习'; return;
      }
      isBreathing = true; btn.textContent='停止';
      const patterns = {
        '478': [{c:'inhale',d:4000,t:'吸气...'},{c:'hold',d:7000,t:'屏息...'},{c:'exhale',d:8000,t:'呼气...'}],
        'box': [{c:'inhale',d:4000,t:'吸气...'},{c:'hold',d:4000,t:'屏息...'},{c:'exhale',d:4000,t:'呼气...'},{c:'hold',d:4000,t:'屏息...'}],
        'calm': [{c:'inhale',d:4000,t:'吸气...'},{c:'hold',d:2000,t:'...'},{c:'exhale',d:6000,t:'呼气...'}]
      };
      const seq = patterns[document.getElementById('breath-pattern').value]||patterns['478'];
      let step = 0;
      function run() {
        if (!isBreathing) return;
        const p = seq[step % seq.length];
        circle.className = `breath-circle ${p.c}`;
        text.textContent = p.t;
        step++;
        breathingInterval = setTimeout(run, p.d);
      }
      run();
    });

    // New affirmation
    document.getElementById('btn-new-affirmation')?.addEventListener('click', () => UI.renderAffirmation());

    // Voice training
    this.setupTagToggles('voice-log-tags', true);
    document.getElementById('btn-save-voice-log')?.addEventListener('click', () => {
      const content = document.getElementById('voice-log-content').value.trim();
      const tags = [...document.querySelectorAll('#voice-log-tags .tag.selected')].map(t=>t.dataset.tag);
      const duration = document.getElementById('voice-log-duration').value;
      if (!content && tags.length === 0) return UI.toast('记录点什么吧','error');
      Store.addVoiceLog({ content, tags, duration: +duration || 0 });
      UI.toast('训练日志已保存 🎤','success');
      document.getElementById('voice-log-content').value='';
      document.getElementById('voice-log-duration').value='';
      document.querySelectorAll('#voice-log-tags .tag').forEach(t=>t.classList.remove('selected'));
      UI.renderVoiceLogs();
    });

    // Pitch monitoring (basic Web Audio API)
    let pitchActive = false, audioCtx, analyser, animFrame;
    document.getElementById('btn-start-pitch')?.addEventListener('click', async () => {
      if (pitchActive) {
        pitchActive = false;
        cancelAnimationFrame(animFrame);
        audioCtx?.close();
        document.getElementById('btn-start-pitch').classList.remove('hidden');
        document.getElementById('btn-stop-pitch').classList.add('hidden');
        document.getElementById('pitch-value').textContent = '-- Hz';
        return;
      }
      try {
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        audioCtx = new AudioContext();
        analyser = audioCtx.createAnalyser();
        analyser.fftSize = 2048;
        const source = audioCtx.createMediaStreamSource(stream);
        source.connect(analyser);
        pitchActive = true;
        document.getElementById('btn-start-pitch').classList.add('hidden');
        document.getElementById('btn-stop-pitch').classList.remove('hidden');
        document.getElementById('btn-stop-pitch').addEventListener('click', () => {
          pitchActive = false; cancelAnimationFrame(animFrame); audioCtx.close();
          document.getElementById('btn-start-pitch').classList.remove('hidden');
          document.getElementById('btn-stop-pitch').classList.add('hidden');
          document.getElementById('pitch-value').textContent = '-- Hz';
          stream.getTracks().forEach(t=>t.stop());
        });
        const buf = new Float32Array(analyser.fftSize);
        function detect() {
          if (!pitchActive) return;
          analyser.getFloatTimeDomainData(buf);
          // Simple autocorrelation
          let rms = 0;
          for (let i = 0; i < buf.length; i++) rms += buf[i]*buf[i];
          rms = Math.sqrt(rms/buf.length);
          if (rms > 0.01) {
            // Find pitch via zero crossings
            let crossings = 0;
            for (let i = 1; i < buf.length; i++) {
              if ((buf[i] >= 0 && buf[i-1] < 0) || (buf[i] < 0 && buf[i-1] >= 0)) crossings++;
            }
            const freq = crossings * audioCtx.sampleRate / (2 * buf.length);
            if (freq > 60 && freq < 500) {
              document.getElementById('pitch-value').textContent = `${Math.round(freq)} Hz`;
              const indicator = document.getElementById('pitch-indicator');
              if (indicator) {
                const pct = Math.min(100, Math.max(0, (freq - 80) / (300 - 80) * 100));
                indicator.style.bottom = `${pct}%`;
              }
            }
          }
          animFrame = requestAnimationFrame(detect);
        }
        detect();
      } catch (err) {
        UI.toast('无法访问麦克风，请检查权限', 'error');
      }
    });

    // Skin care
    document.querySelectorAll('.routine-tabs .tag').forEach(tag => {
      tag.addEventListener('click', () => {
        document.querySelectorAll('.routine-tabs .tag').forEach(t=>t.classList.remove('active'));
        tag.classList.add('active');
        UI.renderSkinRoutine(tag.dataset.routine);
      });
    });
    document.querySelectorAll('#skin-rating .mood-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('#skin-rating .mood-btn').forEach(b=>b.classList.remove('selected'));
        btn.classList.add('selected');
      });
    });
    this.setupTagToggles('skin-issues');
    document.getElementById('btn-save-skin')?.addEventListener('click', () => {
      const sel = document.querySelector('#skin-rating .mood-btn.selected');
      if (!sel) return UI.toast('请选择皮肤状态','error');
      Store.addSkinLog({ rating: +sel.dataset.rating, issues: [...document.querySelectorAll('#skin-issues .tag.selected')].map(t=>t.dataset.tag), note: document.getElementById('skin-note').value });
      UI.toast('皮肤记录已保存 🧴','success');
      document.querySelectorAll('#skin-rating .mood-btn').forEach(b=>b.classList.remove('selected'));
      document.querySelectorAll('#skin-issues .tag').forEach(t=>t.classList.remove('selected'));
      document.getElementById('skin-note').value='';
    });
    document.getElementById('btn-add-product')?.addEventListener('click', () => {
      const name = document.getElementById('product-name').value.trim();
      if (!name) return;
      Store.addProduct({ name, type: document.getElementById('product-type').value });
      document.getElementById('product-name').value='';
      UI.renderProducts(); UI.toast('产品已添加','success');
    });

    // Makeup
    this.setupTagToggles('makeup-techniques');
    document.querySelectorAll('#makeup-satisfaction .mood-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('#makeup-satisfaction .mood-btn').forEach(b=>b.classList.remove('selected'));
        btn.classList.add('selected');
      });
    });
    document.getElementById('btn-save-makeup')?.addEventListener('click', () => {
      const sel = document.querySelector('#makeup-satisfaction .mood-btn.selected');
      Store.addMakeupLog({
        style: document.getElementById('makeup-style').value,
        techniques: [...document.querySelectorAll('#makeup-techniques .tag.selected')].map(t=>t.dataset.tag),
        satisfaction: sel ? +sel.dataset.rating : 0,
        note: document.getElementById('makeup-note').value
      });
      UI.toast('妆容已记录 💄','success');
      document.querySelectorAll('#makeup-techniques .tag').forEach(t=>t.classList.remove('selected'));
      document.querySelectorAll('#makeup-satisfaction .mood-btn').forEach(b=>b.classList.remove('selected'));
      document.getElementById('makeup-note').value='';
      UI.renderMakeupHistory();
    });

    // Fashion
    this.setupTagToggles('outfit-styles');
    document.querySelectorAll('#outfit-confidence .mood-btn').forEach(btn => {
      btn.addEventListener('click', () => {
        document.querySelectorAll('#outfit-confidence .mood-btn').forEach(b=>b.classList.remove('selected'));
        btn.classList.add('selected');
      });
    });
    document.getElementById('btn-save-outfit')?.addEventListener('click', () => {
      const sel = document.querySelector('#outfit-confidence .mood-btn.selected');
      Store.addOutfitLog({
        occasion: document.getElementById('outfit-occasion').value,
        desc: document.getElementById('outfit-desc').value,
        styles: [...document.querySelectorAll('#outfit-styles .tag.selected')].map(t=>t.dataset.tag),
        confidence: sel ? +sel.dataset.rating : 0,
        note: document.getElementById('outfit-note').value
      });
      UI.toast('穿搭已记录 👗','success');
      document.querySelectorAll('#outfit-styles .tag').forEach(t=>t.classList.remove('selected'));
      document.querySelectorAll('#outfit-confidence .mood-btn').forEach(b=>b.classList.remove('selected'));
      document.getElementById('outfit-desc').value='';
      document.getElementById('outfit-note').value='';
      UI.renderOutfitHistory();
    });

    // Posture
    document.getElementById('btn-save-posture')?.addEventListener('click', () => {
      const content = document.getElementById('posture-log').value.trim();
      if (!content) return UI.toast('记录点什么吧','error');
      Store.addPostureLog({ content, duration: +document.getElementById('posture-duration').value || 0 });
      UI.toast('仪态训练已保存','success');
      document.getElementById('posture-log').value='';
      document.getElementById('posture-duration').value='';
      UI.renderPostureLogs();
    });

    // Exercise
    document.querySelectorAll('.workout-tabs .tag').forEach(tag => {
      tag.addEventListener('click', () => {
        document.querySelectorAll('.workout-tabs .tag').forEach(t=>t.classList.remove('active'));
        tag.classList.add('active');
        UI.renderWorkoutPlan(tag.dataset.workout);
      });
    });
    this.setupTagToggles('workout-status', true);
    document.getElementById('btn-save-workout')?.addEventListener('click', () => {
      const content = document.getElementById('workout-log').value.trim();
      if (!content) return UI.toast('记录训练内容','error');
      const tags = [...document.querySelectorAll('#workout-status .tag.selected')].map(t=>t.dataset.tag);
      Store.addWorkoutLog({ content, duration: +document.getElementById('workout-duration').value || 0, tags });
      UI.toast('训练已保存 💪','success');
      document.getElementById('workout-log').value='';
      document.getElementById('workout-duration').value='';
      document.querySelectorAll('#workout-status .tag').forEach(t=>t.classList.remove('selected'));
      UI.renderWorkoutHistory();
    });

    // Routine add steps
    document.getElementById('btn-add-morning')?.addEventListener('click', () => {
      const input = document.getElementById('add-morning-step');
      const text = input.value.trim(); if (!text) return;
      const steps = Store.getRoutine('morning');
      steps.push({ text, done: false });
      Store.setRoutine('morning', steps);
      input.value = ''; UI.renderRoutine('morning');
    });
    document.getElementById('btn-add-evening')?.addEventListener('click', () => {
      const input = document.getElementById('add-evening-step');
      const text = input.value.trim(); if (!text) return;
      const steps = Store.getRoutine('evening');
      steps.push({ text, done: false });
      Store.setRoutine('evening', steps);
      input.value = ''; UI.renderRoutine('evening');
    });

    // Body journal
    document.getElementById('body-journal-date').value = new Date().toISOString().split('T')[0];
    this.setupTagToggles('body-feeling', true);
    this.setupTagToggles('body-areas');
    document.getElementById('btn-save-body-journal')?.addEventListener('click', () => {
      const content = document.getElementById('body-journal-content').value.trim();
      if (!content) return UI.toast('写点什么吧','error');
      const feeling = document.querySelector('#body-feeling .tag.selected')?.dataset.tag || '';
      const areas = [...document.querySelectorAll('#body-areas .tag.selected')].map(t=>t.dataset.tag);
      Store.addBodyJournal({ content, feeling, areas, date: document.getElementById('body-journal-date').value });
      UI.toast('身体日记已保存 📓','success');
      document.getElementById('body-journal-content').value='';
      document.querySelectorAll('#body-feeling .tag, #body-areas .tag').forEach(t=>t.classList.remove('selected'));
      UI.renderBodyJournals();
    });

    // Stories
    document.getElementById('btn-share-story')?.addEventListener('click', () => {
      const content = document.getElementById('story-content').value.trim();
      if (!content) return UI.toast('写点什么吧','error');
      Store.addStory({ category: document.getElementById('story-category').value, content, anon: true });
      UI.toast('已匿名分享 🌻','success');
      document.getElementById('story-content').value='';
      UI.renderStories();
    });
    document.querySelectorAll('.story-filters .tag').forEach(tag => {
      tag.addEventListener('click', () => {
        document.querySelectorAll('.story-filters .tag').forEach(t=>t.classList.remove('active'));
        tag.classList.add('active');
        UI.renderStories(tag.dataset.filter);
      });
    });
  }
};

// SW registration
if ('serviceWorker' in navigator) navigator.serviceWorker.register('sw.js').catch(()=>{});

// Init
document.addEventListener('DOMContentLoaded', () => Prism.init());
