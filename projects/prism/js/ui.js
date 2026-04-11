/* Prism UI - Rendering layer */
const UI = {
  toast(msg, type='info', dur=3000) {
    let c = document.querySelector('.toast-container');
    if (!c) { c = document.createElement('div'); c.className = 'toast-container'; document.body.appendChild(c); }
    const t = document.createElement('div'); t.className = `toast ${type}`; t.textContent = msg; c.appendChild(t);
    setTimeout(() => { t.style.opacity='0'; t.style.transform='translateY(20px)'; setTimeout(() => t.remove(), 300); }, dur);
  },

  renderGreeting() {
    const h = new Date().getHours();
    const el = document.getElementById('greeting');
    if (h < 6) el.textContent = '夜深了 🌙';
    else if (h < 12) el.textContent = '早安 ☀️';
    else if (h < 18) el.textContent = '下午好 🌤️';
    else el.textContent = '晚上好 🌙';
  },

  renderStreak() {
    document.getElementById('streak-number').textContent = Store.getStreak();
    const cal = Store.getCalendarData();
    document.getElementById('streak-calendar').innerHTML = cal.map(d =>
      `<div class="streak-day ${d.active ? 'active' : ''} ${d.isToday ? 'today' : ''}" title="${d.date}"></div>`
    ).join('');
  },

  // Dashboard summaries
  renderDashboardRoutine() {
    const today = new Date().toISOString().split('T')[0];
    const morning = Store.getRoutine('morning');
    const done = Store.getRoutineCompletions()[today] || {};
    const totalSteps = morning.length;
    const completedSteps = (done.morning || []).length;
    const el = document.getElementById('dashboard-routine');
    el.innerHTML = `
      <div style="display:flex;align-items:center;gap:12px;margin-bottom:8px">
        <div class="goal-mini-bar" style="flex:1;height:8px"><div class="goal-mini-bar-fill" style="width:${totalSteps?completedSteps/totalSteps*100:0}%"></div></div>
        <span style="font-size:0.85rem;color:var(--text-muted)">${completedSteps}/${totalSteps}</span>
      </div>
      <p class="text-muted">${completedSteps === totalSteps ? '✅ 晨间流程已完成！' : `还有 ${totalSteps - completedSteps} 步未完成`}</p>
    `;
  },

  renderDashboardTraining() {
    const logs = [...Store.getVoiceLogs(), ...Store.getWorkoutLogs(), ...Store.getPostureLogs()]
      .filter(l => l.ts && l.ts.startsWith(new Date().toISOString().split('T')[0]));
    const el = document.getElementById('dashboard-training');
    if (logs.length === 0) {
      el.innerHTML = '<p class="text-muted">今天还没有训练记录</p>';
    } else {
      el.innerHTML = logs.map(l => `<div style="font-size:0.85rem;padding:4px 0">✅ ${l.content ? l.content.slice(0,40) : '已记录'}</div>`).join('');
    }
  },

  renderDashboardSkin() {
    const logs = Store.getSkinLogs();
    const today = logs.find(l => l.ts && l.ts.startsWith(new Date().toISOString().split('T')[0]));
    const el = document.getElementById('dashboard-skin');
    if (today) {
      const emojis = ['','😣','😕','😐','🙂','😊'];
      el.innerHTML = `<div style="text-align:center;font-size:2rem">${emojis[today.rating]||'😐'}</div><p class="text-muted" style="text-align:center">${(today.issues||[]).join('、') || '状态不错'}</p>`;
    } else {
      el.innerHTML = '<p class="text-muted">今天还没记录皮肤状态</p>';
    }
  },

  renderDashboardOutfit() {
    const logs = Store.getOutfitLogs();
    const today = logs.find(l => l.ts && l.ts.startsWith(new Date().toISOString().split('T')[0]));
    const el = document.getElementById('dashboard-outfit');
    if (today) {
      el.innerHTML = `<p style="font-size:0.9rem">${today.desc ? today.desc.slice(0,60) : '已记录'}</p><div style="margin-top:4px">${(today.styles||[]).map(s=>`<span class="tag selected">${s}</span>`).join('')}</div>`;
    } else {
      el.innerHTML = '<p class="text-muted">今天还没记录穿搭</p>';
    }
  },

  // Voice log list
  renderVoiceLogs() {
    const logs = Store.getVoiceLogs().slice(0, 20);
    const el = document.getElementById('voice-log-list');
    if (logs.length === 0) { el.innerHTML = '<p class="text-muted">还没有训练日志</p>'; return; }
    el.innerHTML = logs.map(l => {
      const d = new Date(l.ts);
      return `<div class="mood-entry"><div class="mood-entry-content">
        <div class="mood-entry-date">${d.toLocaleDateString('zh-CN')} · ${l.duration||0}分钟</div>
        <div class="mood-entry-tags">${(l.tags||[]).map(t=>`<span class="tag">${t}</span>`).join('')}</div>
        ${l.content?`<p style="font-size:0.8rem;margin-top:4px;color:var(--text-secondary)">${l.content}</p>`:''}
      </div></div>`;
    }).join('');
  },

  // Skin routine render
  renderSkinRoutine(type='morning') {
    const steps = PrismData.skinRoutines[type];
    const el = document.getElementById('skin-routine');
    el.innerHTML = steps.map((s, i) => `
      <div class="selfcare-task" onclick="this.classList.toggle('done')">
        <div class="selfcare-task-check"></div>
        <div class="selfcare-task-text">
          <strong>${s.step}</strong>
          <div class="text-muted" style="font-size:0.75rem">${s.tip}</div>
        </div>
      </div>
    `).join('');
  },

  // Products
  renderProducts() {
    const products = Store.getProducts();
    const el = document.getElementById('product-shelf');
    if (products.length === 0) { el.innerHTML = '<p class="text-muted">添加你使用的产品</p>'; return; }
    el.innerHTML = products.map(p => `
      <div class="strategy-item">
        <div class="strategy-content">
          <div class="strategy-title">${p.name}</div>
          <span class="strategy-tag">${PrismData.productTypes[p.type]||p.type}</span>
        </div>
        <button class="btn-icon" onclick="Store.removeProduct('${p.id}');UI.renderProducts()" title="删除">🗑️</button>
      </div>
    `).join('');
  },

  // Skin history
  renderSkinHistory() {
    const logs = Store.getSkinLogs().slice(0, 20);
    const el = document.getElementById('skin-history') || document.querySelector('#page-skin .skin-layout');
    // embedded in skin page, not separate card
  },

  // Makeup history
  renderMakeupHistory() {
    const logs = Store.getMakeupLogs().slice(0, 20);
    const el = document.getElementById('makeup-history');
    if (logs.length === 0) { el.innerHTML = '<p class="text-muted">还没有妆容记录</p>'; return; }
    const emojis = ['','😣','😕','😐','🙂','😍'];
    el.innerHTML = logs.map(l => {
      const d = new Date(l.ts);
      return `<div class="mood-entry">
        <span class="mood-entry-emoji">${emojis[l.satisfaction]||'😐'}</span>
        <div class="mood-entry-content">
          <div class="mood-entry-date">${d.toLocaleDateString('zh-CN')} · ${l.style}</div>
          <div class="mood-entry-tags">${(l.techniques||[]).map(t=>`<span class="tag">${t}</span>`).join('')}</div>
          ${l.note?`<p style="font-size:0.8rem;margin-top:4px">${l.note}</p>`:''}
        </div>
      </div>`;
    }).join('');
  },

  // Outfit history
  renderOutfitHistory() {
    const logs = Store.getOutfitLogs().slice(0, 20);
    const el = document.getElementById('outfit-history');
    if (logs.length === 0) { el.innerHTML = '<p class="text-muted">还没有穿搭记录</p>'; return; }
    const emojis = ['','😰','😕','😐','😊','💃'];
    el.innerHTML = logs.map(l => {
      const d = new Date(l.ts);
      return `<div class="mood-entry">
        <span class="mood-entry-emoji">${emojis[l.confidence]||'😐'}</span>
        <div class="mood-entry-content">
          <div class="mood-entry-date">${d.toLocaleDateString('zh-CN')} · ${l.occasion}</div>
          <div class="mood-entry-tags">${(l.styles||[]).map(s=>`<span class="tag">${s}</span>`).join('')}</div>
          ${l.desc?`<p style="font-size:0.8rem;margin-top:4px">${l.desc.slice(0,80)}</p>`:''}
        </div>
      </div>`;
    }).join('');
  },

  // Posture logs
  renderPostureLogs() {
    const logs = Store.getPostureLogs().slice(0, 20);
    const el = document.getElementById('posture-log-list');
    if (logs.length === 0) { el.innerHTML = '<p class="text-muted">还没有训练日志</p>'; return; }
    el.innerHTML = logs.map(l => {
      const d = new Date(l.ts);
      return `<div class="mood-entry"><div class="mood-entry-content">
        <div class="mood-entry-date">${d.toLocaleDateString('zh-CN')} · ${l.duration||0}分钟</div>
        ${l.content?`<p style="font-size:0.8rem;color:var(--text-secondary)">${l.content}</p>`:''}
      </div></div>`;
    }).join('');
  },

  // Workout plan
  renderWorkoutPlan(type='full') {
    const exercises = PrismData.workouts[type] || PrismData.workouts.full;
    const el = document.getElementById('workout-plan');
    el.innerHTML = exercises.map(e => `
      <div class="exercise-step">
        <div class="exercise-num">💪</div>
        <div>
          <strong>${e.name}</strong> <span class="text-muted">${e.reps}</span>
          <p class="text-muted">${e.desc}</p>
        </div>
      </div>
    `).join('');
  },

  // Workout history
  renderWorkoutHistory() {
    const logs = Store.getWorkoutLogs().slice(0, 20);
    const el = document.getElementById('workout-history');
    if (logs.length === 0) { el.innerHTML = '<p class="text-muted">还没有训练记录</p>'; return; }
    el.innerHTML = logs.map(l => {
      const d = new Date(l.ts);
      return `<div class="mood-entry"><div class="mood-entry-content">
        <div class="mood-entry-date">${d.toLocaleDateString('zh-CN')} · ${l.duration||0}分钟</div>
        <div class="mood-entry-tags">${(l.tags||[]).map(t=>`<span class="tag">${t}</span>`).join('')}</div>
        ${l.content?`<p style="font-size:0.8rem;color:var(--text-secondary)">${l.content}</p>`:''}
      </div></div>`;
    }).join('');
  },

  // Routines
  renderRoutine(type) {
    const steps = Store.getRoutine(type);
    const today = new Date().toISOString().split('T')[0];
    const completions = Store.getRoutineCompletions()[today] || {};
    const done = completions[type] || [];
    const el = document.getElementById(`${type}-routine`);
    el.innerHTML = steps.map((s, i) => `
      <div class="selfcare-task ${done.includes(i)?'done':''}" onclick="Prism.toggleRoutineStep('${type}',${i},this)">
        <div class="selfcare-task-check"></div>
        <span class="selfcare-task-text">${s.text}</span>
      </div>
    `).join('');
  },

  renderRoutineStats() {
    const completions = Store.getRoutineCompletions();
    const days = Object.keys(completions).sort().reverse().slice(0, 14);
    const el = document.getElementById('routine-stats');
    el.innerHTML = days.map(d => {
      const morning = (completions[d]?.morning || []).length;
      const evening = (completions[d]?.evening || []).length;
      const mTotal = Store.getRoutine('morning').length;
      const eTotal = Store.getRoutine('evening').length;
      return `<div style="display:flex;align-items:center;gap:12px;padding:8px 0;border-bottom:1px solid var(--border-light)">
        <span style="font-size:0.85rem;min-width:80px">${d}</span>
        <div class="goal-mini-bar" style="flex:1"><div class="goal-mini-bar-fill" style="width:${mTotal?morning/mTotal*100:0}%;background:var(--coral)"></div></div>
        <span class="text-muted" style="font-size:0.75rem">☀️${morning}/${mTotal}</span>
        <div class="goal-mini-bar" style="flex:1"><div class="goal-mini-bar-fill" style="width:${eTotal?evening/eTotal*100:0}%;background:var(--purple)"></div></div>
        <span class="text-muted" style="font-size:0.75rem">🌙${evening}/${eTotal}</span>
      </div>`;
    }).join('') || '<p class="text-muted">还没有打卡记录</p>';
  },

  // Body journal
  renderBodyJournals() {
    const journals = Store.getBodyJournals().slice(0, 30);
    const el = document.getElementById('body-journal-list');
    if (journals.length === 0) { el.innerHTML = '<div class="empty-state"><div class="empty-state-icon">📓</div><div class="empty-state-text">还没有身体日记</div></div>'; return; }
    el.innerHTML = journals.map(j => {
      const d = new Date(j.ts);
      return `<div class="journal-entry">
        <div class="journal-entry-title">${d.toLocaleDateString('zh-CN')} · ${j.feeling || ''}</div>
        <div class="journal-entry-preview">${(j.content||'').slice(0,80)}...</div>
        <div class="mood-entry-tags" style="margin-top:4px">${(j.areas||[]).map(a=>`<span class="tag">${a}</span>`).join('')}</div>
      </div>`;
    }).join('');
  },

  // Stories
  renderStories(filter='all') {
    let stories = Store.getStories();
    if (filter !== 'all') stories = stories.filter(s => s.category === filter);
    const el = document.getElementById('stories-feed');
    el.innerHTML = stories.map(s => {
      const liked = Store.isLiked(s.id);
      return `<div class="story-card">
        <div class="story-meta">
          <span class="story-author">匿名 · ${s.date}</span>
          <span class="story-cat">${PrismData.storyCategories[s.category]||s.category}</span>
        </div>
        <div class="story-text">${s.content}</div>
        <div class="story-actions">
          <button class="story-action ${liked?'liked':''}" onclick="Prism.likeStory('${s.id}',this)">${liked?'❤️':'🤍'} ${s.likes||0}</button>
        </div>
      </div>`;
    }).join('');
  },

  // Self-care checklist (wellness page)
  renderSelfCareCheck() {
    const today = new Date().toISOString().split('T')[0];
    const done = (Store.getSelfCareCheck()[today]) || [];
    const el = document.getElementById('selfcare-checklist');
    el.innerHTML = PrismData.selfCareItems.map(item => `
      <div class="selfcare-task ${done.includes(item)?'done':''}" onclick="Prism.toggleSelfCare(this, '${item.replace(/'/g,"\\'")}')">
        <div class="selfcare-task-check"></div>
        <span class="selfcare-task-text">${item}</span>
      </div>
    `).join('');
  },

  // Affirmation
  renderAffirmation() {
    const all = PrismData.affirmations;
    const el = document.getElementById('daily-affirmation');
    el.innerHTML = `<p style="font-size:1.1rem;font-style:italic;line-height:1.8">"${all[Math.floor(Math.random()*all.length)]}"</p>`;
  },

  // Mood chart (simple canvas)
  renderMoodChart() {
    // Optional - can add mood chart on wellness page later
  }
};
