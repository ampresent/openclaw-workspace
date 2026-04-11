# JazzLab — 爵士练习工作台

一个面向爵士乐爱好者和练习者的全面 Web 应用，集成和弦/音阶参考、互动钢琴、摇摆节拍器、听音训练、标准曲库等功能。

## 功能模块

### 🎹 互动钢琴
- 两个八度的虚拟钢琴（C3-B4）
- 电脑键盘映射（A-L 白键，W-E-T-Y-U 黑键）
- 和弦/音阶高亮可视化
- 实时 Web Audio 多振荡器丰富音色（三角波 + 谐波 + 低通滤波 + 压缩器）

### 🎵 和弦库
- 30+ 种爵士和弦类型（大七、属七、小七、减七、增七、挂留、Altered 等）
- 按分类筛选（大/属/小/减/增/挂留）
- 12 个根音自由切换
- 点击播放和弦声音

### 🎼 音阶库
- 教会调式（7 个模式）
- 旋律小调体系（7 个模式）
- 和声小调
- 对称音阶（全音阶、减音阶）
- Bebop 音阶
- 五声音阶、布鲁斯音阶、半音阶
- 点击播放音阶上行

### 🎸 吉他指板
- 标准调弦（EADGBE）15 品指板
- 可视化和弦指法和音阶指型
- 点击品位发声

### 🥁 摇摆节拍器
- BPM 范围 40-300
- 拍号支持：2/4, 3/4, 4/4, 5/4, 6/8, 7/8
- Swing Feel：Straight / Light / Medium / Hard / Shuffle
- Tap Tempo（自动检测速度）
- 可视化节拍环动画

### 🎶 Comping 节奏模式库
- 8 种经典爵士伴奏节奏型
- Swing Comping / Charleston / Off-beat / Freddie Green
- Latin / Bossa Nova / Sparse / Double-time
- 可视化节奏模式图
- 点击即可跟着节拍器练习

### 👂 听音训练（4 模式）
- **和弦性质识别**：随机播放和弦，选择正确类型
- **音程识别**：听两个音，判断音程距离
- **和弦进行识别**：听一组和弦，判断进行类型
- **II-V-I 辨调**：听 II-V-I 进行，识别调性
- 正确率/连续正确数统计 + 进度条

### 📖 爵士标准曲库
- 22 首经典爵士标准曲（含 Giant Steps、Wave、Misty 等）
- 完整和弦进行（含段落标注）
- 按曲式筛选（AABA / ABAC / AB / Blues）
- 搜索功能
- 播放和弦进行

### 📝 练习日志
- 练习计时器（开始/暂停/重置）
- 按类型分类记录（音阶/和弦/即兴/曲目/听力/乐理）
- 本地 localStorage 持久化存储

### 📚 爵士乐理指南
- II-V-I 进行详解
- 三全音替代
- Voice Leading 与 Guide Tones
- 副属和弦
- 调式理论（教会调式 + 旋律小调）
- Voicing 技术（Shell / Drop 2 / Rootless）
- 重配和声技巧
- 即兴演奏方法论

### ⭕ 五度圈
- 可视化五度圈
- 点击调性查看相关和弦（I-ii-iii-IV-V-vi-vii°）
- 关系小调
- II-V-I 进行推荐

## 技术栈

- **纯前端**：HTML + CSS + JavaScript，无需构建工具
- **Web Audio API**：多振荡器合成 + 低通滤波 + 动态压缩器
- **本地存储**：localStorage 持久化练习记录
- **响应式设计**：支持桌面和移动端

## 运行方式

直接用浏览器打开 `index.html` 即可，或使用任何静态文件服务器：

```bash
cd projects/jazzlab
python3 -m http.server 8080
# 访问 http://localhost:8080
```

## 项目结构

```
jazzlab/
├── index.html      # 主应用（含所有 HTML + JS，~1700 行）
├── style.css       # 样式文件（~600 行）
├── README.md       # 本文件
└── PLAN.md         # 项目规划与迭代计划
```

## 待迭代方向

- [ ] 添加更多音色（电钢琴 Wurlitzer/Rhodes、贝斯、吉他）
- [ ] 自动伴奏引擎（Swing Bass + Comping pattern）
- [ ] ii-V-I 练习器（自动伴奏 + 即兴提示）
- [ ] Transcription Helper（慢速播放 + 循环片段）
- [ ] 录音功能
- [ ] 导入/导出练习记录
- [ ] MIDI 键盘支持
- [ ] 更多标准曲目（目标 50+）
- [ ] Chord Voicing 展示（Drop 2, Drop 3, Block chords）
- [ ] 练习统计仪表板
- [ ] PWA 离线支持
