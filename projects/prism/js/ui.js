/* Prism UI v0.3.0 — Proactive Today-first rendering */
const UI = {
  toast(msg, type='info', dur=3000) {
    let c = document.querySelector('.toast-container');
    if (!c) { c = document.createElement('div'); c.className = 'toast-container'; document.body.appendChild(c); }
    const t = document.createElement('div'); t.className = `toast ${type}`; t.textContent = msg; c.appendChild(t);
    setTimeout(() => { t.style.opacity='0'; setTimeout(() => t.remove(), 300); }, dur);
  },

  // ============================
  // TODAY VIEW — the proactive core
  // ============================
  renderToday() {
    this.renderGreeting();
    this.renderTodayStats();
    this.renderMorningPlan();
    this.renderTrainingPlan();
    this.renderSkinPlan();
    this.renderWellnessPlan();
    this.renderEveningPlan();
  },

  renderGreeting() {
    const h = new Date().getHours();
    const el = document.getElementById('greeting-time');
    const sub = document.getElementById('greeting-sub');
    const dow = Store.dayOfWeek();
    const dayNames = ['周日','周一','周二','周三','周四','周五','周六'];
    const weekPlan = PrismData.weeklyPlan[dow];

    if (h < 6) { el.textContent = '夜深了 🌙'; sub.textContent = '好好休息，明天又是新的一天。'; }
    else if (h < 9) { el.textContent = '早安 ☀️'; sub.innerHTML = `${dayNames}，${weekPlan.label}。今天为自己做一件小事吧。`; }
    else if (h < 12) { el.textContent = '上午好 🌤️'; sub.innerHTML = `${dayNames} · ${weekPlan.icon} ${weekPlan.label}`; }
    else if (h < 18) { el.textContent = '下午好 🌤️'; sub.innerHTML = `${dayNames} · ${weekPlan.icon} ${weekPlan.label}`; }
    else { el.textContent = '晚上好 🌙'; sub.textContent = '辛苦了一天，别忘了照顾自己。'; }

    // Affirmation
    const affirmEl = document.getElementById('daily-affirmation');
    const all = PrismData.affirmations;
    affirmEl.textContent = `"${all[Math.floor(Math.random() * all.length)]}"`;

    // Streak
    document.getElementById('streak-number').textContent = Store.getStreak();
  },

  renderTodayStats() {
    const stats = Store.getTodayStats();
    const el = document.getElementById('today-stats');
    // Render a progress ring
    const circumference = 2 * Math.PI * 22;
    const offset = circumference - (stats.pct / 100) * circumference;
    el.innerHTML = `
      <div class="progress-ring">
        <svg width="56" height="56" viewBox="0 0 56 56">
          <circle class="progress-ring-circle" cx="28" cy="28" r="22"/>
          <circle class="progress-ring-fill" cx="28" cy="28" r="22"
            stroke-dasharray="${circumference}" stroke-dashoffset="${offset}"/>
        </svg>
        <div class="progress-ring-text">${stats.pct}%</div>
      </div>
      <div class="stats-bar">
        <div class="stat-pill">☀️ 晨间 <span class="num">${stats.morning.done}/${stats.morning.total}</span></div>
        <div class="stat-pill">${stats.hasVoice ? '✅' : '⬜'} 声音</div>
        <div class="stat-pill">${stats.hasWorkout ? '✅' : '⬜'} 锻炼</div>
        <div class="stat-pill">${stats.hasSkin ? '✅' : '⬜'} 护肤</div>
      </div>
    `;
  },

  // ---- MORNING ROUTINE PLAN ----
  renderMorningPlan() {
    const progress = Store.getRoutineProgressToday('morning');
    const done = progress.pct >= 100;
    const steps = Store.getRoutine('morning');
    const today = Store.today();
    const completions = Store.getRoutineCompletions()[today] || {};
    const doneIdx = completions.morning || [];

    const el = document.getElementById('plan-morning');
    el.className = `plan-card ${done ? 'completed' : ''}`;
    el.style.setProperty('--card-accent', 'var(--gold)');
    el.style.setProperty('--card-accent-dim', 'var(--gold-dim)');

    el.innerHTML = `
      <div class="plan-header" onclick="App.togglePlan('plan-morning')">
        <div class="plan-icon">☀️</div>
        <div class="plan-meta">
          <div class="plan-title">晨间流程 ${done ? '<span class="badge">已完成</span>' : ''}</div>
          <div class="plan-time">${progress.done}/${progress.total} 步骤</div>
        </div>
        <div class="plan-check ${done ? 'checked' : ''}" id="check-morning">${done ? '✓' : ''}</div>
      </div>
      <div class="routine-bar"><div class="routine-bar-fill" style="width:${progress.pct}%;background:var(--gold)"></div></div>
      <div class="plan-detail" id="detail-plan-morning">
        <div class="plan-detail-inner">
          <ul class="plan-steps">
            ${steps.map((s, i) => `
              <li class="plan-step ${doneIdx.includes(i) ? 'done' : ''}" onclick="App.toggleMorningStep(${i}, this)">
                <span class="plan-step-check">${doneIdx.includes(i) ? '✓' : ''}</span>
                <span>${s.text}</span>
              </li>
            `).join('')}
          </ul>
        </div>
      </div>
    `;

    // Auto-expand if not done
    if (!done && progress.pct < 100) {
      setTimeout(() => {
        document.getElementById('detail-plan-morning')?.classList.add('open');
      }, 500);
    }
  },

  // ---- TRAINING PLAN (voice/body based on day) ----
  renderTrainingPlan() {
    const dow = Store.dayOfWeek();
    const weekPlan = PrismData.weeklyPlan[dow];
    const today = Store.today();

    if (weekPlan.type === 'rest') {
      this._renderRestDay();
      return;
    }

    const isVoice = weekPlan.type === 'voice';
    const hasLogged = isVoice
      ? Store.getVoiceLogs().some(l => l.ts && l.ts.startsWith(today))
      : Store.getWorkoutLogs().some(l => l.ts && l.ts.startsWith(today));

    const sectionTitle = document.getElementById('section-training');
    sectionTitle.textContent = isVoice ? '// 今日训练 · 声音' : '// 今日训练 · 身体';

    const el = document.getElementById('plan-training');
    el.className = `plan-card ${hasLogged ? 'completed' : ''}`;
    const accent = isVoice ? 'var(--coral)' : 'var(--teal)';
    const accentDim = isVoice ? 'var(--coral-dim)' : 'var(--teal-dim)';
    el.style.setProperty('--card-accent', accent);
    el.style.setProperty('--card-accent-dim', accentDim);

    if (isVoice) {
      const sentence = PrismData.voicePracticeSentences[Math.floor(Math.random() * PrismData.voicePracticeSentences.length)];
      el.innerHTML = `
        <div class="plan-header" onclick="App.togglePlan('plan-training')">
          <div class="plan-icon">🎤</div>
          <div class="plan-meta">
            <div class="plan-title">声音训练 ${hasLogged ? '<span class="badge">已记录</span>' : ''}</div>
            <div class="plan-time">建议 15-20 分钟</div>
          </div>
          <div class="plan-check ${hasLogged ? 'checked' : ''}">${hasLogged ? '✓' : ''}</div>
        </div>
        <div class="plan-detail" id="detail-plan-training">
          <div class="plan-detail-inner">
            <p style="font-size:0.85rem;color:var(--text-secondary);margin-bottom:14px">今天的练习句子：</p>
            <div style="padding:14px 18px;background:var(--bg-input);border-radius:var(--radius-sm);font-size:0.95rem;margin-bottom:16px;font-style:italic;color:var(--text)">"${sentence}"</div>
            <div class="pitch-monitor" id="pitch-monitor">
              <div class="pitch-value" id="pitch-value">-- Hz</div>
              <div class="pitch-label">实时音高</div>
              <div class="pitch-indicator" id="pitch-indicator"></div>
            </div>
            <button class="plan-action" id="btn-start-pitch" onclick="App.startPitchMonitor()">🎤 开始监测</button>
            <button class="plan-action hidden" id="btn-stop-pitch" onclick="App.stopPitchMonitor()" style="background:var(--coral)">⏹ 停止</button>
            <div class="quick-log" style="margin-top:16px">
              <input type="number" placeholder="时长(分钟)" id="voice-duration" style="max-width:120px">
              <input type="text" placeholder="训练笔记..." id="voice-note">
              <button class="btn btn-sm btn-primary" onclick="App.saveVoiceLog()">记录</button>
            </div>
          </div>
        </div>
      `;
      if (!hasLogged) {
        setTimeout(() => document.getElementById('detail-plan-training')?.classList.add('open'), 800);
      }
    } else {
      // Body training — pick today's workout type
      const workoutKeys = ['full', 'lower', 'core', 'upper', 'yoga'];
      const workoutIdx = (Math.floor((new Date() - new Date(2026, 0, 1)) / 86400000)) % workoutKeys.length;
      const workoutKey = workoutKeys[workoutIdx];
      const workout = PrismData.workouts[workoutKey];

      el.innerHTML = `
        <div class="plan-header" onclick="App.togglePlan('plan-training')">
          <div class="plan-icon">💪</div>
          <div class="plan-meta">
            <div class="plan-title">${workout.name} ${hasLogged ? '<span class="badge">已记录</span>' : ''}</div>
            <div class="plan-time">约 ${workout.duration} 分钟 · ${workout.exercises.length} 个动作</div>
          </div>
          <div class="plan-check ${hasLogged ? 'checked' : ''}">${hasLogged ? '✓' : ''}</div>
        </div>
        <div class="plan-detail" id="detail-plan-training">
          <div class="plan-detail-inner">
            ${workout.exercises.map(e => `
              <div class="exercise-step">
                <div class="exercise-num">${e.icon}</div>
                <div class="exercise-info">
                  <strong>${e.name}</strong> <span class="exercise-reps">${e.reps}</span>
                  <div class="exercise-desc">${e.desc}</div>
                </div>
              </div>
            `).join('')}
            <div class="quick-log" style="margin-top:16px">
              <input type="number" placeholder="时长(分钟)" id="workout-duration" style="max-width:120px" value="${workout.duration}">
              <input type="text" placeholder="感受笔记..." id="workout-note">
              <button class="btn btn-sm btn-primary" onclick="App.saveWorkoutLog('${workoutKey}')">记录完成</button>
            </div>
          </div>
        </div>
      `;
      if (!hasLogged) {
        setTimeout(() => document.getElementById('detail-plan-training')?.classList.add('open'), 800);
      }
    }
  },

  _renderRestDay() {
    document.getElementById('section-training').textContent = '// 今日主题 · 休息与回顾';
    const el = document.getElementById('plan-training');
    el.className = 'plan-card';
    el.style.setProperty('--card-accent', 'var(--purple)');
    el.style.setProperty('--card-accent-dim', 'var(--purple-dim)');

    const logs = [...Store.getVoiceLogs(), ...Store.getWorkoutLogs()].slice(0, 5);
    el.innerHTML = `
      <div class="plan-header" onclick="App.togglePlan('plan-training')">
        <div class="plan-icon">🌸</div>
        <div class="plan-meta">
          <div class="plan-title">周日回顾</div>
          <div class="plan-time">休息也是进步的一部分</div>
        </div>
      </div>
      <div class="plan-detail" id="detail-plan-training">
        <div class="plan-detail-inner">
          <p style="font-size:0.85rem;color:var(--text-secondary);margin-bottom:12px">最近的训练记录：</p>
          ${logs.length === 0 ? '<p class="text-muted">还没有训练记录，从下周一开始吧！</p>' :
            logs.map(l => `<div class="entry-item"><div class="entry-date">${new Date(l.ts).toLocaleDateString('zh-CN')}</div><div class="entry-content">${(l.content || l.tags?.join(', ') || '已训练').slice(0, 60)}</div></div>`).join('')
          }
        </div>
      </div>
    `;
  },

  // ---- SKIN PLAN ----
  renderSkinPlan() {
    const today = Store.today();
    const hasSkin = Store.getSkinLogs().some(l => l.ts && l.ts.startsWith(today));
    const h = new Date().getHours();
    const routineType = h >= 17 ? 'evening' : 'morning';
    const routine = PrismData.skinRoutines[routineType];
    const label = routineType === 'morning' ? '晨间护肤' : '夜间护肤';

    const el = document.getElementById('plan-skin');
    el.className = `plan-card ${hasSkin ? 'completed' : ''}`;
    el.style.setProperty('--card-accent', 'var(--blue)');
    el.style.setProperty('--card-accent-dim', 'var(--blue-dim)');

    el.innerHTML = `
      <div class="plan-header" onclick="App.togglePlan('plan-skin')">
        <div class="plan-icon">🧴</div>
        <div class="plan-meta">
          <div class="plan-title">${label} ${hasSkin ? '<span class="badge">已记录</span>' : ''}</div>
          <div class="plan-time">${routine.length} 步骤 · 约 ${Math.ceil(routine.reduce((a, r) => a + r.seconds, 0) / 60)} 分钟</div>
        </div>
        <div class="plan-check ${hasSkin ? 'checked' : ''}">${hasSkin ? '✓' : ''}</div>
      </div>
      <div class="plan-detail" id="detail-plan-skin">
        <div class="plan-detail-inner">
          <ul class="plan-steps">
            ${routine.map(s => `
              <li class="plan-step" onclick="this.classList.toggle('done');this.querySelector('.plan-step-check').textContent=this.classList.contains('done')?'✓':''">
                <span class="plan-step-check"></span>
                <span><strong>${s.step}</strong> <span style="color:var(--text-muted);font-size:0.8rem">${s.tip}</span></span>
              </li>
            `).join('')}
          </ul>
          <div class="mood-scale" style="margin-top:16px;justify-content:center">
            ${['😣','😕','😐','🙂','😊'].map((e, i) => `<button class="mood-btn" data-rating="${i+1}" onclick="App.selectSkinRating(this)">${e}</button>`).join('')}
          </div>
          <div class="quick-log" style="margin-top:12px">
            <input type="text" placeholder="皮肤状态备注..." id="skin-note">
            <button class="btn btn-sm btn-primary" onclick="App.saveSkinLog()">记录</button>
          </div>
        </div>
      </div>
    `;
  },

  // ---- WELLNESS PLAN ----
  renderWellnessPlan() {
    const today = Store.today();
    const hasMood = Store.getMoods().some(l => l.ts && l.ts.startsWith(today));
    const el = document.getElementById('plan-wellness');
    el.className = `plan-card ${hasMood ? 'completed' : ''}`;
    el.style.setProperty('--card-accent', 'var(--purple)');
    el.style.setProperty('--card-accent-dim', 'var(--purple-dim)');

    el.innerHTML = `
      <div class="plan-header" onclick="App.togglePlan('plan-wellness')">
        <div class="plan-icon">💜</div>
        <div class="plan-meta">
          <div class="plan-title">心情签到 ${hasMood ? '<span class="badge">已签</span>' : ''}</div>
          <div class="plan-time">花 10 秒钟记录一下</div>
        </div>
        <div class="plan-check ${hasMood ? 'checked' : ''}">${hasMood ? '✓' : ''}</div>
      </div>
      <div class="plan-detail" id="detail-plan-wellness">
        <div class="plan-detail-inner">
          <div style="text-align:center;margin-bottom:12px;font-size:0.85rem;color:var(--text-muted)">今天心情怎么样？</div>
          <div class="mood-scale" style="justify-content:center">
            ${['😣','😕','😐','🙂','😊','😄'].map((e, i) => `<button class="mood-btn" data-mood="${i+1}" onclick="App.selectMood(this)">${e}</button>`).join('')}
          </div>
          <div id="mood-detail" class="hidden" style="margin-top:14px">
            <div class="tags-row" id="mood-tags" style="justify-content:center;margin-bottom:10px">
              ${['平静','开心','焦虑','疲惫','期待','孤独','自信','感动'].map(t => `<span class="tag" data-tag="${t}" onclick="this.classList.toggle('selected')">${t}</span>`).join('')}
            </div>
            <div class="quick-log">
              <input type="text" placeholder="想说点什么..." id="mood-note">
              <button class="btn btn-sm btn-primary" onclick="App.saveMood()">记录</button>
            </div>
          </div>
        </div>
      </div>
    `;
  },

  // ---- EVENING PLAN ----
  renderEveningPlan() {
    const h = new Date().getHours();
    const progress = Store.getRoutineProgressToday('evening');
    const done = progress.pct >= 100;
    const steps = Store.getRoutine('evening');
    const today = Store.today();
    const completions = Store.getRoutineCompletions()[today] || {};
    const doneIdx = completions.evening || [];

    const el = document.getElementById('plan-evening');
    el.className = `plan-card ${done ? 'completed' : ''}`;
    el.style.setProperty('--card-accent', 'var(--purple)');
    el.style.setProperty('--card-accent-dim', 'var(--purple-dim)');

    // Only show evening section if it's afternoon/evening or if already started
    const section = document.getElementById('section-evening');
    if (h >= 16 || doneIdx.length > 0) {
      section.style.display = '';
    } else {
      section.style.display = 'none';
    }

    el.innerHTML = `
      <div class="plan-header" onclick="App.togglePlan('plan-evening')">
        <div class="plan-icon">🌙</div>
        <div class="plan-meta">
          <div class="plan-title">晚间流程 ${done ? '<span class="badge">已完成</span>' : ''}</div>
          <div class="plan-time">${progress.done}/${progress.total} 步骤</div>
        </div>
        <div class="plan-check ${done ? 'checked' : ''}">${done ? '✓' : ''}</div>
      </div>
      <div class="routine-bar"><div class="routine-bar-fill" style="width:${progress.pct}%;background:var(--purple)"></div></div>
      <div class="plan-detail" id="detail-plan-evening">
        <div class="plan-detail-inner">
          <ul class="plan-steps">
            ${steps.map((s, i) => `
              <li class="plan-step ${doneIdx.includes(i) ? 'done' : ''}" onclick="App.toggleEveningStep(${i}, this)">
                <span class="plan-step-check">${doneIdx.includes(i) ? '✓' : ''}</span>
                <span>${s.text}</span>
              </li>
            `).join('')}
          </ul>
        </div>
      </div>
    `;
  },

  // ============================
  // SUB-PAGE RENDERERS (when user drills into a module)
  // ============================

  renderVoicePage() {
    const logs = Store.getVoiceLogs().slice(0, 20);
    const el = document.getElementById('voice-log-list');
    if (logs.length === 0) { el.innerHTML = '<div class="empty-state"><div class="empty-state-icon">🎤</div><div class="empty-state-text">还没有训练记录</div></div>'; return; }
    el.innerHTML = logs.map(l => {
      const d = new Date(l.ts);
      return `<div class="entry-item"><div class="entry-date">${d.toLocaleDateString('zh-CN')} · ${l.duration||0}分钟</div><div class="entry-tags">${(l.tags||[]).map(t=>`<span class="tag">${t}</span>`).join('')}</div>${l.content?`<div class="entry-content">${l.content}</div>`:''}</div>`;
    }).join('');
  },

  renderSkinPage() {
    const type = new Date().getHours() >= 17 ? 'evening' : 'morning';
    const routine = PrismData.skinRoutines[type];
    const el = document.getElementById('skin-routine-full');
    el.innerHTML = routine.map(s => `
      <div class="exercise-step">
        <div class="exercise-num">🧴</div>
        <div class="exercise-info">
          <strong>${s.step}</strong>
          <div class="exercise-desc">${s.tip}</div>
        </div>
      </div>
    `).join('');
    this.renderProducts();
  },

  renderProducts() {
    const products = Store.getProducts();
    const el = document.getElementById('product-shelf');
    if (products.length === 0) { el.innerHTML = '<p style="color:var(--text-muted);font-size:0.85rem">还没有添加产品</p>'; return; }
    el.innerHTML = products.map(p => `
      <div class="entry-item" style="display:flex;justify-content:space-between;align-items:center">
        <div><strong style="font-size:0.9rem">${p.name}</strong> <span class="tag">${PrismData.productTypes[p.type]||p.type}</span></div>
        <button class="btn btn-ghost btn-sm" onclick="Store.removeProduct('${p.id}');UI.renderProducts()">🗑️</button>
      </div>
    `).join('');
  },

  renderExercisePage() {
    const el = document.getElementById('workout-plan-full');
    const workoutKeys = ['full', 'lower', 'core', 'upper', 'yoga'];
    el.innerHTML = workoutKeys.map(k => {
      const w = PrismData.workouts[k];
      return `<div class="card">
        <div class="card-title">${w.name} <span class="tag">${w.duration}分钟</span></div>
        ${w.exercises.map(e => `<div class="exercise-step"><div class="exercise-num">${e.icon}</div><div class="exercise-info"><strong>${e.name}</strong> <span class="exercise-reps">${e.reps}</span><div class="exercise-desc">${e.desc}</div></div></div>`).join('')}
      </div>`;
    }).join('');
  },

  renderMakeupPage() {
    const logs = Store.getMakeupLogs().slice(0, 20);
    const el = document.getElementById('makeup-history');
    if (logs.length === 0) { el.innerHTML = '<div class="empty-state"><div class="empty-state-icon">💄</div><div class="empty-state-text">还没有妆容记录</div></div>'; return; }
    const emojis = ['','😣','😕','😐','🙂','😍'];
    el.innerHTML = logs.map(l => {
      const d = new Date(l.ts);
      return `<div class="entry-item"><div class="entry-date">${d.toLocaleDateString('zh-CN')} · ${l.style} ${emojis[l.satisfaction]||''}</div><div class="entry-tags">${(l.techniques||[]).map(t=>`<span class="tag">${t}</span>`).join('')}</div>${l.note?`<div class="entry-content">${l.note}</div>`:''}</div>`;
    }).join('');
  },

  renderFashionPage() {
    const logs = Store.getOutfitLogs().slice(0, 20);
    const el = document.getElementById('outfit-history');
    if (logs.length === 0) { el.innerHTML = '<div class="empty-state"><div class="empty-state-icon">👗</div><div class="empty-state-text">还没有穿搭记录</div></div>'; return; }
    const emojis = ['','😰','😕','😐','😊','💃'];
    el.innerHTML = logs.map(l => {
      const d = new Date(l.ts);
      return `<div class="entry-item"><div class="entry-date">${d.toLocaleDateString('zh-CN')} · ${l.occasion} ${emojis[l.confidence]||''}</div><div class="entry-tags">${(l.styles||[]).map(s=>`<span class="tag">${s}</span>`).join('')}</div>${l.desc?`<div class="entry-content">${l.desc}</div>`:''}</div>`;
    }).join('');
  },

  renderJournalPage() {
    const journals = Store.getBodyJournals().slice(0, 20);
    const el = document.getElementById('body-journal-list');
    if (journals.length === 0) { el.innerHTML = '<div class="empty-state"><div class="empty-state-icon">📓</div><div class="empty-state-text">还没有身体日记</div></div>'; return; }
    el.innerHTML = journals.map(j => {
      const d = new Date(j.ts);
      return `<div class="entry-item"><div class="entry-date">${d.toLocaleDateString('zh-CN')} · ${j.feeling||''}</div><div class="entry-content">${(j.content||'').slice(0,100)}</div><div class="entry-tags">${(j.areas||[]).map(a=>`<span class="tag">${a}</span>`).join('')}</div></div>`;
    }).join('');
  },

  renderCommunityPage(filter='all') {
    let stories = Store.getStories();
    if (filter !== 'all') stories = stories.filter(s => s.category === filter);
    const el = document.getElementById('stories-feed');
    el.innerHTML = stories.map(s => {
      const liked = Store.isLiked(s.id);
      return `<div class="story-card"><div class="story-meta"><span>匿名 · ${s.date}</span><span>${PrismData.storyCategories[s.category]||s.category}</span></div><div class="story-text">${s.content}</div><div class="story-actions"><button class="story-action ${liked?'liked':''}" onclick="App.likeStory('${s.id}',this)">${liked?'❤️':'🤍'} ${s.likes||0}</button></div></div>`;
    }).join('');
  }
};
