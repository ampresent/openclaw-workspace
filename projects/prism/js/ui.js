/* Prism UI v0.4.0 — Action-first, no logging */
const UI = {
  toast(msg, type='info', dur=2500) {
    let c = document.querySelector('.toast-container');
    if (!c) { c = document.createElement('div'); c.className = 'toast-container'; document.body.appendChild(c); }
    const t = document.createElement('div'); t.className = `toast ${type}`; t.textContent = msg; c.appendChild(t);
    setTimeout(() => { t.style.opacity='0'; setTimeout(() => t.remove(), 300); }, dur);
  },

  // ============================
  // TODAY VIEW
  // ============================
  renderToday() {
    this.renderGreeting();
    this.renderMorningCard();
    this.renderTrainingCard();
    this.renderSkinCard();
    this.renderOutfitCard();
    this.renderEveningCard();
  },

  renderGreeting() {
    const h = Store.hour();
    const el = document.getElementById('greeting-time');
    const sub = document.getElementById('greeting-sub');
    const dow = Store.dayOfWeek();
    const dayNames = ['周日','周一','周二','周三','周四','周五','周六'];
    const weekPlan = PrismData.weeklyPlan[dow];

    if (h < 6) { el.textContent = '夜深了 🌙'; sub.textContent = '好好休息。'; }
    else if (h < 9) { el.textContent = '早安 ☀️'; sub.textContent = `${dayNames} · ${weekPlan.label}`; }
    else if (h < 12) { el.textContent = '上午好 🌤️'; sub.textContent = `${dayNames} · ${weekPlan.label}`; }
    else if (h < 18) { el.textContent = '下午好'; sub.textContent = `${dayNames} · ${weekPlan.label}`; }
    else { el.textContent = '晚上好 🌙'; sub.textContent = `${dayNames} · ${weekPlan.label}`; }

    // Affirmation
    const affirmEl = document.getElementById('daily-affirmation');
    const all = PrismData.affirmations;
    const seed = new Date().getDate() % all.length;
    affirmEl.textContent = `"${all[seed]}"`;

    // Streak
    document.getElementById('streak-number').textContent = Store.getStreak();
  },

  // ---- MORNING ROUTINE (timer-guided) ----
  renderMorningCard() {
    const steps = [
      "起床喝水一杯",
      "洁面 + 晨间护肤",
      "整理发型",
      "选择今日穿搭",
      "基础妆容",
    ];
    const done = Store.getRoutineProgressToday('morning');
    const complete = done.length >= steps.length;
    const pct = Math.round(done.length / steps.length * 100);

    const el = document.getElementById('plan-morning');
    el.className = `plan-card ${complete ? 'completed' : ''}`;
    el.style.setProperty('--card-accent', 'var(--gold)');
    el.style.setProperty('--card-accent-dim', 'var(--gold-dim)');

    el.innerHTML = `
      <div class="plan-header" onclick="App.togglePlan('morning')">
        <div class="plan-icon">☀️</div>
        <div class="plan-meta">
          <div class="plan-title">晨间流程 ${complete ? '<span class="badge">完成</span>' : ''}</div>
          <div class="plan-time">${done.length}/${steps.length} · ${complete ? '干得漂亮' : '点击展开'}</div>
        </div>
        <div class="plan-check ${complete ? 'checked' : ''}">${complete ? '✓' : ''}</div>
      </div>
      <div class="routine-bar"><div class="routine-bar-fill" style="width:${pct}%;background:var(--gold)"></div></div>
      <div class="plan-detail" id="detail-morning">
        <div class="plan-detail-inner">
          <ul class="plan-steps">
            ${steps.map((s, i) => `
              <li class="plan-step ${done.includes(i) ? 'done' : ''}" onclick="App.toggleStep('morning', ${i}, this)">
                <span class="plan-step-check">${done.includes(i) ? '✓' : ''}</span>
                <span>${s}</span>
              </li>
            `).join('')}
          </ul>
        </div>
      </div>
    `;

    if (!complete && done.length === 0) {
      setTimeout(() => document.getElementById('detail-morning')?.classList.add('open'), 400);
    }
  },

  // ---- TRAINING (auto-generated, no logging) ----
  renderTrainingCard() {
    const dow = Store.dayOfWeek();
    const weekPlan = PrismData.weeklyPlan[dow];
    const section = document.getElementById('section-training');

    if (weekPlan.type === 'rest') {
      section.textContent = '// 今日主题';
      const el = document.getElementById('plan-training');
      el.className = 'plan-card';
      el.style.setProperty('--card-accent', 'var(--purple)');
      el.style.setProperty('--card-accent-dim', 'var(--purple-dim)');
      el.innerHTML = `
        <div class="plan-header">
          <div class="plan-icon">🌸</div>
          <div class="plan-meta">
            <div class="plan-title">休息日</div>
            <div class="plan-time">身体在休息中变强，别有负罪感</div>
          </div>
        </div>
      `;
      return;
    }

    const isVoice = weekPlan.type === 'voice';
    section.textContent = isVoice ? '// 声音训练' : '// 身体锻炼';
    const accent = isVoice ? 'var(--coral)' : 'var(--teal)';
    const accentDim = isVoice ? 'var(--coral-dim)' : 'var(--teal-dim)';

    const el = document.getElementById('plan-training');
    el.className = 'plan-card';
    el.style.setProperty('--card-accent', accent);
    el.style.setProperty('--card-accent-dim', accentDim);

    if (isVoice) {
      // Pick 2-3 sentences for today
      const all = PrismData.voicePracticeSentences;
      const dayIdx = new Date().getDate();
      const sentences = [all[dayIdx % all.length], all[(dayIdx + 3) % all.length], all[(dayIdx + 7) % all.length]];

      el.innerHTML = `
        <div class="plan-header" onclick="App.togglePlan('training')">
          <div class="plan-icon">🎤</div>
          <div class="plan-meta">
            <div class="plan-title">声音训练 · 共鸣 & 音高</div>
            <div class="plan-time">建议 15-20 分钟</div>
          </div>
        </div>
        <div class="plan-detail" id="detail-training">
          <div class="plan-detail-inner">
            <p style="font-size:0.8rem;color:var(--text-muted);margin-bottom:10px">练习目标：把共鸣从胸腔移到头部，保持放松</p>
            <div class="pitch-monitor" id="pitch-monitor">
              <div class="pitch-value" id="pitch-value">-- Hz</div>
              <div class="pitch-label">实时音高</div>
              <div class="pitch-indicator" id="pitch-indicator"></div>
            </div>
            <div style="display:flex;gap:8px;margin:12px 0;justify-content:center">
              <button class="btn btn-primary" onclick="App.startPitch()">🎤 开始</button>
              <button class="btn btn-ghost hidden" id="btn-stop-pitch" onclick="App.stopPitch()">⏹ 停止</button>
            </div>
            <p style="font-size:0.8rem;color:var(--text-muted);margin:14px 0 8px">今天的练习句子：</p>
            ${sentences.map(s => `<div class="practice-sentence">"${s}"</div>`).join('')}
          </div>
        </div>
      `;
    } else {
      // Body training
      const workoutKeys = ['full', 'lower', 'core', 'upper', 'yoga'];
      const dayIdx = Math.floor((new Date() - new Date(2026, 0, 1)) / 86400000);
      const workoutKey = workoutKeys[dayIdx % workoutKeys.length];
      const workout = PrismData.workouts[workoutKey];

      el.innerHTML = `
        <div class="plan-header" onclick="App.togglePlan('training')">
          <div class="plan-icon">💪</div>
          <div class="plan-meta">
            <div class="plan-title">${workout.name}</div>
            <div class="plan-time">约 ${workout.duration} 分钟 · ${workout.exercises.length} 个动作</div>
          </div>
        </div>
        <div class="plan-detail" id="detail-training">
          <div class="plan-detail-inner">
            <div id="workout-flow"></div>
          </div>
        </div>
      `;

      // Render workout flow with per-exercise timer
      setTimeout(() => {
        const flow = document.getElementById('workout-flow');
        if (!flow) return;
        flow.innerHTML = workout.exercises.map((e, i) => `
          <div class="exercise-step" id="exercise-${i}">
            <div class="exercise-num">${e.icon}</div>
            <div class="exercise-info" style="flex:1">
              <div style="display:flex;justify-content:space-between;align-items:center">
                <div>
                  <strong>${e.name}</strong> <span class="exercise-reps">${e.reps}</span>
                  <div class="exercise-desc">${e.desc}</div>
                </div>
                <button class="btn btn-sm btn-primary" onclick="App.startExerciseTimer(${i}, ${e.seconds})" id="exercise-btn-${i}">
                  ⏱ ${Math.floor(e.seconds/60)}:${(e.seconds%60).toString().padStart(2,'0')}
                </button>
              </div>
              <div class="exercise-timer-bar hidden" id="exercise-bar-${i}">
                <div class="exercise-timer-fill" id="exercise-fill-${i}"></div>
              </div>
            </div>
          </div>
        `).join('');
      }, 100);
    }

    // Auto-expand
    setTimeout(() => document.getElementById('detail-training')?.classList.add('open'), 600);
  },

  // ---- SKIN CARE (timer-guided steps) ----
  renderSkinCard() {
    const h = Store.hour();
    const routineType = h >= 17 ? 'evening' : 'morning';
    const routine = PrismData.skinRoutines[routineType];
    const label = routineType === 'morning' ? '晨间护肤' : '夜间护肤';
    const totalSec = routine.reduce((a, r) => a + r.seconds, 0);

    const el = document.getElementById('plan-skin');
    el.className = 'plan-card';
    el.style.setProperty('--card-accent', 'var(--blue)');
    el.style.setProperty('--card-accent-dim', 'var(--blue-dim)');

    el.innerHTML = `
      <div class="plan-header" onclick="App.togglePlan('skin')">
        <div class="plan-icon">🧴</div>
        <div class="plan-meta">
          <div class="plan-title">${label}</div>
          <div class="plan-time">${routine.length} 步骤 · 约 ${Math.ceil(totalSec/60)} 分钟</div>
        </div>
      </div>
      <div class="plan-detail" id="detail-skin">
        <div class="plan-detail-inner">
          <div id="skin-flow"></div>
        </div>
      </div>
    `;

    setTimeout(() => {
      const flow = document.getElementById('skin-flow');
      if (!flow) return;
      flow.innerHTML = routine.map((s, i) => `
        <div class="exercise-step" id="skin-step-${i}">
          <div class="exercise-num">🧴</div>
          <div class="exercise-info" style="flex:1">
            <div style="display:flex;justify-content:space-between;align-items:center">
              <div>
                <strong>${s.step}</strong>
                <div class="exercise-desc">${s.tip}</div>
              </div>
              <button class="btn btn-sm btn-primary" onclick="App.startSkinTimer(${i}, ${s.seconds})" id="skin-btn-${i}">
                ⏱ ${s.seconds}s
              </button>
            </div>
            <div class="exercise-timer-bar hidden" id="skin-bar-${i}">
              <div class="exercise-timer-fill" id="skin-fill-${i}"></div>
            </div>
          </div>
        </div>
      `).join('');
    }, 100);
  },

  // ---- OUTFIT SUGGESTION (auto-generated) ----
  renderOutfitCard() {
    const season = PrismData.getSeason();
    const suggestions = PrismData.outfitSuggestions[season];
    const dayIdx = new Date().getDate();
    const pick = suggestions[dayIdx % suggestions.length];

    const el = document.getElementById('plan-outfit');
    el.className = 'plan-card';
    el.style.setProperty('--card-accent', 'var(--gold)');
    el.style.setProperty('--card-accent-dim', 'var(--gold-dim)');

    el.innerHTML = `
      <div class="plan-header" onclick="App.togglePlan('outfit')">
        <div class="plan-icon">👗</div>
        <div class="plan-meta">
          <div class="plan-title">今日穿搭建议 · ${PrismData.seasonLabels[season]}</div>
          <div class="plan-time">${pick.note}</div>
        </div>
      </div>
      <div class="plan-detail" id="detail-outfit">
        <div class="plan-detail-inner">
          <div class="outfit-items">
            ${pick.items.map(item => `<div class="outfit-item"><span class="outfit-item-icon">·</span> ${item}</div>`).join('')}
          </div>
          <p style="font-size:0.8rem;color:var(--text-muted);margin-top:12px;font-style:italic">${pick.note}</p>
          <button class="btn btn-ghost btn-sm" style="margin-top:10px" onclick="App.shuffleOutfit()">🔄 换一套</button>
        </div>
      </div>
    `;
  },

  // ---- EVENING ROUTINE ----
  renderEveningCard() {
    const h = Store.hour();
    const section = document.getElementById('section-evening');
    const el = document.getElementById('plan-evening');

    if (h < 16) {
      section.style.display = 'none';
      el.style.display = 'none';
      return;
    }
    section.style.display = '';
    el.style.display = '';

    const steps = [
      "卸妆",
      "夜间护肤流程",
      "拉伸放松",
      "准备明天的衣服",
      "11点前上床",
    ];
    const done = Store.getRoutineProgressToday('evening');
    const complete = done.length >= steps.length;
    const pct = Math.round(done.length / steps.length * 100);

    el.className = `plan-card ${complete ? 'completed' : ''}`;
    el.style.setProperty('--card-accent', 'var(--purple)');
    el.style.setProperty('--card-accent-dim', 'var(--purple-dim)');

    el.innerHTML = `
      <div class="plan-header" onclick="App.togglePlan('evening')">
        <div class="plan-icon">🌙</div>
        <div class="plan-meta">
          <div class="plan-title">晚间流程 ${complete ? '<span class="badge">完成</span>' : ''}</div>
          <div class="plan-time">${done.length}/${steps.length}</div>
        </div>
        <div class="plan-check ${complete ? 'checked' : ''}">${complete ? '✓' : ''}</div>
      </div>
      <div class="routine-bar"><div class="routine-bar-fill" style="width:${pct}%;background:var(--purple)"></div></div>
      <div class="plan-detail" id="detail-evening">
        <div class="plan-detail-inner">
          <ul class="plan-steps">
            ${steps.map((s, i) => `
              <li class="plan-step ${done.includes(i) ? 'done' : ''}" onclick="App.toggleStep('evening', ${i}, this)">
                <span class="plan-step-check">${done.includes(i) ? '✓' : ''}</span>
                <span>${s}</span>
              </li>
            `).join('')}
          </ul>
        </div>
      </div>
    `;
  },

  // ============================
  // SUB-PAGES (browsable tools)
  // ============================

  renderVoicePage() {
    const all = PrismData.voicePracticeSentences;
    const el = document.getElementById('voice-sentences');
    el.innerHTML = all.map(s =>
      `<div class="practice-sentence">"${s}"</div>`
    ).join('');
  },

  renderSkinPage() {
    const h = Store.hour();
    const type = h >= 17 ? 'evening' : 'morning';
    const routine = PrismData.skinRoutines[type];
    const label = type === 'morning' ? '晨间护肤' : '夜间护肤';
    const totalSec = routine.reduce((a, r) => a + r.seconds, 0);

    document.getElementById('skin-page-title').textContent = `🧴 ${label}`;
    const el = document.getElementById('skin-page-routine');
    el.innerHTML = routine.map((s, i) => `
      <div class="exercise-step" id="skin-page-step-${i}">
        <div class="exercise-num">🧴</div>
        <div class="exercise-info" style="flex:1">
          <div style="display:flex;justify-content:space-between;align-items:center">
            <div>
              <strong>${s.step}</strong>
              <div class="exercise-desc">${s.tip}</div>
            </div>
            <button class="btn btn-sm btn-primary" onclick="App.startSkinTimer('page-${i}', ${s.seconds})" id="skin-page-btn-${i}">
              ⏱ ${s.seconds}s
            </button>
          </div>
          <div class="exercise-timer-bar hidden" id="skin-page-bar-${i}">
            <div class="exercise-timer-fill" id="skin-page-fill-${i}"></div>
          </div>
        </div>
      </div>
    `).join('');
  },

  renderExercisePage() {
    const workoutKeys = ['full', 'lower', 'core', 'upper', 'yoga'];
    const el = document.getElementById('workout-plan-full');
    el.innerHTML = workoutKeys.map(k => {
      const w = PrismData.workouts[k];
      return `<div class="card">
        <div class="card-title">${w.name} <span class="tag">${w.duration}分钟</span></div>
        ${w.exercises.map(e => `
          <div class="exercise-step">
            <div class="exercise-num">${e.icon}</div>
            <div class="exercise-info">
              <strong>${e.name}</strong> <span class="exercise-reps">${e.reps}</span>
              <div class="exercise-desc">${e.desc}</div>
            </div>
          </div>
        `).join('')}
      </div>`;
    }).join('');
  },

  renderCommunityPage(filter='all') {
    let stories = PrismData.defaultStories;
    if (filter !== 'all') stories = stories.filter(s => s.category === filter);
    const el = document.getElementById('stories-feed');
    el.innerHTML = stories.map(s => `
      <div class="story-card">
        <div class="story-meta">
          <span>匿名 · ${s.date}</span>
          <span>${PrismData.storyCategories[s.category]||s.category}</span>
        </div>
        <div class="story-text">${s.content}</div>
        <div class="story-actions"><span class="story-likes">❤️ ${s.likes}</span></div>
      </div>
    `).join('');
  },
};
