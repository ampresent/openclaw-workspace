/* Prism Store - Data persistence layer */
const Store = {
  _p: 'prism_',
  get(k, d=null) { try { const r = localStorage.getItem(this._p+k); return r ? JSON.parse(r) : d; } catch { return d; } },
  set(k, v) { try { localStorage.setItem(this._p+k, JSON.stringify(v)); } catch(e) { console.warn(e); } },
  remove(k) { localStorage.removeItem(this._p+k); },

  // Moods
  getMoods() { return this.get('moods', []); },
  addMood(e) { const m = this.getMoods(); e.id = Date.now().toString(36); e.ts = new Date().toISOString(); m.unshift(e); this.set('moods', m); return e; },

  // Voice logs
  getVoiceLogs() { return this.get('voiceLogs', []); },
  addVoiceLog(e) { const l = this.getVoiceLogs(); e.id = Date.now().toString(36); e.ts = new Date().toISOString(); l.unshift(e); this.set('voiceLogs', l); return e; },

  // Skin logs
  getSkinLogs() { return this.get('skinLogs', []); },
  addSkinLog(e) { const l = this.getSkinLogs(); e.id = Date.now().toString(36); e.ts = new Date().toISOString(); l.unshift(e); this.set('skinLogs', l); return e; },

  // Products
  getProducts() { return this.get('products', []); },
  addProduct(p) { const ps = this.getProducts(); p.id = Date.now().toString(36); ps.push(p); this.set('products', ps); return p; },
  removeProduct(id) { this.set('products', this.getProducts().filter(p => p.id !== id)); },

  // Makeup logs
  getMakeupLogs() { return this.get('makeupLogs', []); },
  addMakeupLog(e) { const l = this.getMakeupLogs(); e.id = Date.now().toString(36); e.ts = new Date().toISOString(); l.unshift(e); this.set('makeupLogs', l); return e; },

  // Outfit logs
  getOutfitLogs() { return this.get('outfitLogs', []); },
  addOutfitLog(e) { const l = this.getOutfitLogs(); e.id = Date.now().toString(36); e.ts = new Date().toISOString(); l.unshift(e); this.set('outfitLogs', l); return e; },

  // Posture logs
  getPostureLogs() { return this.get('postureLogs', []); },
  addPostureLog(e) { const l = this.getPostureLogs(); e.id = Date.now().toString(36); e.ts = new Date().toISOString(); l.unshift(e); this.set('postureLogs', l); return e; },

  // Workout logs
  getWorkoutLogs() { return this.get('workoutLogs', []); },
  addWorkoutLog(e) { const l = this.getWorkoutLogs(); e.id = Date.now().toString(36); e.ts = new Date().toISOString(); l.unshift(e); this.set('workoutLogs', l); return e; },

  // Body journal
  getBodyJournals() { return this.get('bodyJournals', []); },
  addBodyJournal(e) { const l = this.getBodyJournals(); e.id = Date.now().toString(36); e.ts = new Date().toISOString(); l.unshift(e); this.set('bodyJournals', l); return e; },

  // Stories
  getStories() { return [...(this.get('stories', [])), ...PrismData.defaultStories]; },
  addStory(s) { const st = this.get('stories', []); s.id = 's_'+Date.now().toString(36); s.likes=0; s.date=new Date().toISOString().split('T')[0]; st.unshift(s); this.set('stories', st); return s; },
  likeStory(id) { const l = this.get('liked',[]); if(l.includes(id)) return false; l.push(id); this.set('liked',l); const st=this.get('stories',[]); const s=st.find(x=>x.id===id); if(s){s.likes=(s.likes||0)+1;this.set('stories',st);} return true; },
  isLiked(id) { return this.get('liked',[]).includes(id); },

  // Routines
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

  // Routine completions (per day)
  getRoutineCompletions() { return this.get('routineCompletions', {}); },
  setRoutineCompletion(date, type, completed) {
    const c = this.getRoutineCompletions();
    if (!c[date]) c[date] = {};
    c[date][type] = completed;
    this.set('routineCompletions', c);
  },

  // Self-care checklist (daily)
  getSelfCareCheck() { return this.get('selfCareCheck', {}); },
  toggleSelfCareItem(item) {
    const c = this.getSelfCareCheck();
    const today = new Date().toISOString().split('T')[0];
    if (!c[today]) c[today] = [];
    const idx = c[today].indexOf(item);
    if (idx >= 0) c[today].splice(idx, 1); else c[today].push(item);
    this.set('selfCareCheck', c);
    return idx < 0;
  },

  // Streak
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

  getCalendarData() {
    const logs = [...this.getMoods(), ...this.getSkinLogs(), ...this.getVoiceLogs(), ...this.getWorkoutLogs()];
    const days = [];
    const today = new Date();
    for (let i = 27; i >= 0; i--) {
      const d = new Date(today); d.setDate(d.getDate() - i);
      const ds = d.toISOString().split('T')[0];
      days.push({
        date: ds,
        active: logs.some(l => l.ts && l.ts.startsWith(ds)),
        isToday: i === 0
      });
    }
    return days;
  },

  // Export/Import
  exportAll() {
    const keys = ['moods','voiceLogs','skinLogs','products','makeupLogs','outfitLogs',
      'postureLogs','workoutLogs','bodyJournals','stories','liked',
      'routine_morning','routine_evening','routineCompletions','selfCareCheck'];
    const data = {};
    keys.forEach(k => data[k] = this.get(k));
    data._v = '2.0'; data._exportDate = new Date().toISOString();
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
