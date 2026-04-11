/* Prism Store v0.4.0 — Minimal, no manual logging */
const Store = {
  _p: 'prism_',
  get(k, d=null) { try { const r = localStorage.getItem(this._p+k); return r ? JSON.parse(r) : d; } catch { return d; } },
  set(k, v) { try { localStorage.setItem(this._p+k, JSON.stringify(v)); } catch(e) { console.warn(e); } },

  today() { return new Date().toISOString().split('T')[0]; },
  dayOfWeek() { return new Date().getDay(); },
  hour() { return new Date().getHours(); },

  // Routine step completion (the only user action — clicking a step as done)
  getRoutineCompletions() { return this.get('routineCompletions', {}); },
  getRoutineProgressToday(type) {
    const today = this.today();
    const completions = this.getRoutineCompletions()[today] || {};
    return completions[type] || [];
  },
  toggleRoutineStep(type, idx) {
    const today = this.today();
    const c = this.getRoutineCompletions();
    if (!c[today]) c[today] = {};
    if (!c[today][type]) c[today][type] = [];
    const arr = c[today][type];
    const pos = arr.indexOf(idx);
    if (pos >= 0) arr.splice(pos, 1); else arr.push(idx);
    c[today][type] = arr;
    this.set('routineCompletions', c);
    return arr;
  },
  isRoutineComplete(type, totalSteps) {
    return this.getRoutineProgressToday(type).length >= totalSteps;
  },

  // Streak — calculated from any activity
  getStreak() {
    const c = this.getRoutineCompletions();
    const days = Object.keys(c).sort().reverse();
    if (days.length === 0) return 0;
    let streak = 0;
    const today = this.today();
    for (let i = 0; i < 365; i++) {
      const d = new Date(); d.setDate(d.getDate() - i);
      const ds = d.toISOString().split('T')[0];
      if (c[ds] && (c[ds].morning?.length > 0 || c[ds].evening?.length > 0)) streak++;
      else if (i > 0) break;
    }
    return streak;
  },

  // Export/Import
  exportAll() {
    const data = { routineCompletions: this.get('routineCompletions'), _v: '4.0', _exportDate: new Date().toISOString() };
    return data;
  },
  importAll(data) {
    if (data.routineCompletions) this.set('routineCompletions', data.routineCompletions);
  }
};
