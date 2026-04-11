/* Prism Store v0.3.0 */
const Store = {
  _p: 'prism_',
  get(k, d=null) { try { const r = localStorage.getItem(this._p+k); return r ? JSON.parse(r) : d; } catch { return d; } },
  set(k, v) { try { localStorage.setItem(this._p+k, JSON.stringify(v)); } catch(e) { console.warn(e); } },

  today() { return new Date().toISOString().split('T')[0]; },
  dayOfWeek() { return new Date().getDay(); },

  // ---- Generic log helpers ----
  _logs(key) { return this.get(key, []); },
  _addLog(key, entry) {
    const logs = this._logs(key);
    entry.id = Date.now().toString(36);
    entry.ts = new Date().toISOString();
    logs.unshift(entry);
    this.set(key, logs);
    return entry;
  },

  // Moods
  getMoods() { return this._logs('moods'); },
  addMood(e) { return this._addLog('moods', e); },

  // Voice logs
  getVoiceLogs() { return this._logs('voiceLogs'); },
  addVoiceLog(e) { return this._addLog('voiceLogs', e); },

  // Skin logs
  getSkinLogs() { return this._logs('skinLogs'); },
  addSkinLog(e) { return this._addLog('skinLogs', e); },

  // Products
  getProducts() { return this.get('products', []); },
  addProduct(p) { const ps = this.getProducts(); p.id = Date.now().toString(36); ps.push(p); this.set('products', ps); return p; },
  removeProduct(id) { this.set('products', this.getProducts().filter(p => p.id !== id)); },

  // Makeup logs
  getMakeupLogs() { return this._logs('makeupLogs'); },
  addMakeupLog(e) { return this._addLog('makeupLogs', e); },

  // Outfit logs
  getOutfitLogs() { return this._logs('outfitLogs'); },
  addOutfitLog(e) { return this._addLog('outfitLogs', e); },

  // Posture logs
  getPostureLogs() { return this._logs('postureLogs'); },
  addPostureLog(e) { return this._addLog('postureLogs', e); },

  // Workout logs
  getWorkoutLogs() { return this._logs('workoutLogs'); },
  addWorkoutLog(e) { return this._addLog('workoutLogs', e); },

  // Body journal
  getBodyJournals() { return this._logs('bodyJournals'); },
  addBodyJournal(e) { return this._addLog('bodyJournals', e); },

  // Stories
  getStories() { return [...this.get('stories', []), ...PrismData.defaultStories]; },
  addStory(s) { return this._addLog('stories', s); },
  likeStory(id) {
    const liked = this.get('liked', []);
    if (liked.includes(id)) return false;
    liked.push(id); this.set('liked', liked);
    const stories = this.get('stories', []);
    const s = stories.find(x => x.id === id);
    if (s) { s.likes = (s.likes || 0) + 1; this.set('stories', stories); }
    return true;
  },
  isLiked(id) { return this.get('liked', []).includes(id); },

  // ---- Routines ----
  getRoutine(type) {
    const defaults = {
      morning: [
        { text: "起床喝水一杯", done: false },
        { text: "洁面 + 晨间护肤", done: false },
        { text: "整理发型", done: false },
        { text: "选择今日穿搭", done: false },
        { text: "基础妆容", done: false }
      ],
      evening: [
        { text: "卸妆", done: false },
        { text: "夜间护肤流程", done: false },
        { text: "拉伸放松", done: false },
        { text: "准备明天的衣服", done: false },
        { text: "11点前上床", done: false }
      ]
    };
    return this.get(`routine_${type}`, defaults[type] || []);
  },
  setRoutine(type, data) { this.set(`routine_${type}`, data); },

  getRoutineCompletions() { return this.get('routineCompletions', {}); },
  setRoutineCompletion(date, type, completed) {
    const c = this.getRoutineCompletions();
    if (!c[date]) c[date] = {};
    c[date][type] = completed;
    this.set('routineCompletions', c);
  },
  isRoutineDoneToday(type) {
    const today = this.today();
    const completions = this.getRoutineCompletions()[today] || {};
    const total = this.getRoutine(type).length;
    const done = (completions[type] || []).length;
    return total > 0 && done >= total;
  },
  getRoutineProgressToday(type) {
    const today = this.today();
    const completions = this.getRoutineCompletions()[today] || {};
    const total = this.getRoutine(type).length;
    const done = (completions[type] || []).length;
    return { done, total, pct: total ? Math.round(done / total * 100) : 0 };
  },

  // ---- Today plan completion tracking ----
  getTodayPlan() { return this.get(`plan_${this.today()}`, {}); },
  setTodayPlan(data) { this.set(`plan_${this.today()}`, data); },
  isPlanItemDone(key) { return !!this.getTodayPlan()[key]; },
  togglePlanItem(key) {
    const plan = this.getTodayPlan();
    plan[key] = !plan[key];
    this.setTodayPlan(plan);
    return plan[key];
  },

  // ---- Self-care ----
  getSelfCareCheck() { return this.get('selfCareCheck', {}); },
  toggleSelfCareItem(item) {
    const c = this.getSelfCareCheck();
    const today = this.today();
    if (!c[today]) c[today] = [];
    const idx = c[today].indexOf(item);
    if (idx >= 0) c[today].splice(idx, 1); else c[today].push(item);
    this.set('selfCareCheck', c);
    return idx < 0;
  },

  // ---- Streak ----
  getStreak() {
    const logs = [...this.getMoods(), ...this.getSkinLogs(), ...this.getVoiceLogs(),
      ...this.getWorkoutLogs(), ...this.getMakeupLogs(), ...this.getOutfitLogs(),
      ...this.getPostureLogs(), ...this.getBodyJournals()];
    if (logs.length === 0) return 0;
    let streak = 0;
    const today = new Date(); today.setHours(0,0,0,0);
    for (let i = 0; i < 365; i++) {
      const d = new Date(today); d.setDate(d.getDate() - i);
      const ds = d.toISOString().split('T')[0];
      if (logs.some(l => l.ts && l.ts.startsWith(ds))) streak++;
      else if (i > 0) break;
    }
    return streak;
  },

  // ---- Today completion stats ----
  getTodayStats() {
    const today = this.today();
    const morning = this.getRoutineProgressToday('morning');
    const evening = this.getRoutineProgressToday('evening');
    const hasVoice = this.getVoiceLogs().some(l => l.ts && l.ts.startsWith(today));
    const hasWorkout = this.getWorkoutLogs().some(l => l.ts && l.ts.startsWith(today));
    const hasSkin = this.getSkinLogs().some(l => l.ts && l.ts.startsWith(today));
    const hasMood = this.getMoods().some(l => l.ts && l.ts.startsWith(today));
    const hasJournal = this.getBodyJournals().some(l => l.ts && l.ts.startsWith(today));

    let total = 6, completed = 0;
    if (morning.pct >= 100) completed++;
    if (evening.pct >= 100) completed++;
    if (hasVoice) completed++;
    if (hasWorkout) completed++;
    if (hasSkin) completed++;
    if (hasMood || hasJournal) completed++;

    return { total, completed, pct: Math.round(completed / total * 100), morning, evening, hasVoice, hasWorkout, hasSkin, hasMood, hasJournal };
  },

  // ---- Export / Import ----
  exportAll() {
    const keys = ['moods','voiceLogs','skinLogs','products','makeupLogs','outfitLogs',
      'postureLogs','workoutLogs','bodyJournals','stories','liked',
      'routine_morning','routine_evening','routineCompletions','selfCareCheck'];
    const data = {};
    keys.forEach(k => data[k] = this.get(k));
    data._v = '3.0'; data._exportDate = new Date().toISOString();
    return data;
  },
  importAll(data) {
    if (!data._v) throw new Error('Invalid');
    ['moods','voiceLogs','skinLogs','products','makeupLogs','outfitLogs',
      'postureLogs','workoutLogs','bodyJournals','stories','liked',
      'routine_morning','routine_evening','routineCompletions','selfCareCheck']
      .forEach(k => { if (data[k] !== undefined) this.set(k, data[k]); });
  }
};
