/* Prism Data v0.4.0 */
const PrismData = {
  affirmations: [
    "我的身体是我表达自己的画布。",
    "每一步改变都在让我更接近真实的自己。",
    "我不需要完美，我需要真实。",
    "今天的我比昨天更了解自己。",
    "我有权利以自己舒服的方式存在。",
    "我的声音值得被听见。",
    "美不止一种标准，我定义自己的美。",
    "照顾好身体就是爱自己的方式。",
    "每照一次镜子，试着找到一个喜欢的地方。",
    "改变是一个过程，我有耐心。",
    "今天的练习是为明天的自信投资。",
    "身体的变化需要时间，但每一步都算数。",
    "我值得拥有让自己舒服的身体。",
    "今天也要善待这副陪伴我一生的身体。",
    "每一次练习都在重塑我的可能性。",
  ],

  voicePracticeSentences: [
    "今天天气真不错，我想出去走走。",
    "请问，这个多少钱？",
    "你好，我叫___，很高兴认识你。",
    "我觉得这个想法很好，我们可以试试看。",
    "谢谢你，这对我来说很重要。",
    "我最近在学新的东西，感觉很有意思。",
    "周末有什么计划吗？我想去看电影。",
    "这杯咖啡味道不错，你在哪里买的？",
    "我其实挺喜欢安静的环境。",
    "明天见啦，路上小心哦。",
    "不好意思，能再说一遍吗？",
    "我觉得这个颜色挺好看的。",
  ],

  // Skin routines — each step has a timer duration
  skinRoutines: {
    morning: [
      { step: "洁面", tip: "温水 + 温和洁面乳，打圈 30 秒", seconds: 60 },
      { step: "化妆水", tip: "轻拍至吸收，平衡肌肤 pH", seconds: 45 },
      { step: "精华液", tip: "维 C 精华早间用，抗氧化", seconds: 30 },
      { step: "眼霜", tip: "无名指轻拍眼周，力度要轻", seconds: 30 },
      { step: "面霜", tip: "根据肤质选择质地，均匀涂抹", seconds: 30 },
      { step: "防晒", tip: "SPF30+，一元硬币大小，别忘了脖子", seconds: 45 },
    ],
    evening: [
      { step: "卸妆", tip: "先卸妆再洁面，即使只涂了防晒", seconds: 90 },
      { step: "洁面", tip: "二次清洁，确保干净", seconds: 60 },
      { step: "化妆水", tip: "补水打底", seconds: 45 },
      { step: "精华液", tip: "烟酰胺/视黄醇等活性成分晚上用", seconds: 30 },
      { step: "眼霜", tip: "夜间修复眼周肌肤", seconds: 30 },
      { step: "面霜", tip: "晚间可以用更滋润的质地", seconds: 30 },
    ],
  },

  // Workouts
  workouts: {
    full: {
      name: "全身塑形", duration: 25,
      exercises: [
        { name: "深蹲", reps: "15次×3组", desc: "双脚与肩同宽，臀部向后坐", seconds: 90 },
        { name: "臀桥", reps: "20次×3组", desc: "仰卧屈膝，发力抬起臀部", seconds: 90 },
        { name: "平板支撑", reps: "30秒×3组", desc: "身体一条直线，核心收紧", seconds: 90 },
        { name: "侧抬腿", reps: "每侧15次×3组", desc: "侧卧上方腿伸直抬起", seconds: 90 },
        { name: "俯身划船", reps: "12次×3组", desc: "哑铃或水瓶，收紧背部", seconds: 90 },
        { name: "猫牛式", reps: "10次", desc: "四点跪姿交替弓背塌腰", seconds: 60 },
      ]
    },
    lower: {
      name: "臀腿专项", duration: 30,
      exercises: [
        { name: "深蹲", reps: "15次×4组", desc: "注意臀部发力感", seconds: 120 },
        { name: "臀桥", reps: "20次×4组", desc: "可以单腿增加难度", seconds: 120 },
        { name: "保加利亚分腿蹲", reps: "每侧12次×3组", desc: "后脚放椅子上", seconds: 120 },
        { name: "侧卧蚌式", reps: "每侧20次×3组", desc: "侧卧屈膝，上方膝盖打开", seconds: 90 },
        { name: "消防栓式", reps: "每侧15次×3组", desc: "四点跪姿侧上方抬起", seconds: 90 },
        { name: "小腿提踵", reps: "20次×3组", desc: "站立脚跟抬起放下", seconds: 60 },
      ]
    },
    core: {
      name: "腰腹核心", duration: 20,
      exercises: [
        { name: "平板支撑", reps: "45秒×3组", desc: "核心收紧，不塌腰", seconds: 135 },
        { name: "死虫式", reps: "每侧10次×3组", desc: "仰卧对侧手脚交替", seconds: 90 },
        { name: "卷腹", reps: "15次×3组", desc: "腹部发力，不用脖子", seconds: 90 },
        { name: "俄罗斯转体", reps: "20次×3组", desc: "坐姿微后仰左右转", seconds: 90 },
        { name: "登山者", reps: "30秒×3组", desc: "平板撑交替提膝", seconds: 90 },
        { name: "仰卧骑车", reps: "20次×3组", desc: "仰卧双脚模拟骑车", seconds: 90 },
      ]
    },
    upper: {
      name: "上身柔化", duration: 20,
      exercises: [
        { name: "跪姿俯卧撑", reps: "10次×3组", desc: "膝盖着地降低难度", seconds: 90 },
        { name: "弹力带拉伸", reps: "15次×3组", desc: "开肩展背", seconds: 90 },
        { name: "超人式", reps: "12次×3组", desc: "俯卧同时抬起手脚", seconds: 90 },
        { name: "手臂画圈", reps: "30秒×3组", desc: "双臂伸直画小圈", seconds: 90 },
        { name: "墙壁推", reps: "15次×3组", desc: "面对墙壁推墙", seconds: 60 },
        { name: "肩部拉伸", reps: "每侧30秒", desc: "一手横过胸前辅助拉伸", seconds: 60 },
      ]
    },
    yoga: {
      name: "柔韧瑜伽", duration: 25,
      exercises: [
        { name: "山式站立", reps: "1分钟", desc: "双脚并拢感受平衡", seconds: 60 },
        { name: "战士一式", reps: "每侧30秒", desc: "前弓步双手上举", seconds: 60 },
        { name: "战士二式", reps: "每侧30秒", desc: "侧弓步双臂侧平举", seconds: 60 },
        { name: "三角式", reps: "每侧30秒", desc: "侧弯身一手触脚踝", seconds: 60 },
        { name: "树式", reps: "每侧30秒", desc: "单脚站立一脚抵大腿", seconds: 60 },
        { name: "下犬式", reps: "5次呼吸", desc: "倒V字形", seconds: 30 },
        { name: "桥式", reps: "5次呼吸", desc: "仰卧屈膝抬起臀部", seconds: 30 },
        { name: "婴儿式", reps: "1分钟", desc: "跪坐放松", seconds: 60 },
      ]
    },
  },

  // Weekly training schedule
  weeklyPlan: {
    1: { type: 'voice', label: '声音训练', icon: '🎤' },
    2: { type: 'body',  label: '身体锻炼', icon: '💪' },
    3: { type: 'voice', label: '声音训练', icon: '🎤' },
    4: { type: 'body',  label: '身体锻炼', icon: '💪' },
    5: { type: 'voice', label: '声音训练', icon: '🎤' },
    6: { type: 'body',  label: '身体锻炼', icon: '💪' },
    0: { type: 'rest',  label: '休息日', icon: '🌸' },
  },

  // Outfit suggestions by season/context
  outfitSuggestions: {
    spring: [
      { items: ["薄款针织衫", "高腰阔腿裤", "帆布鞋"], note: "温柔知性风，层次感刚刚好" },
      { items: ["衬衫", "A字半裙", "乐福鞋"], note: "通勤约会两相宜" },
      { items: ["卫衣", "直筒牛仔裤", "小白鞋"], note: "轻松休闲，舒适自在" },
    ],
    summer: [
      { items: ["吊带连衣裙", "凉鞋", "草编包"], note: "清爽夏日，一条裙子搞定" },
      { items: ["T恤", "短裤/短裙", "帆布鞋"], note: "简单舒服，出门快" },
      { items: ["防晒衣", "阔腿裤", "凉鞋"], note: "防晒又好看" },
    ],
    autumn: [
      { items: ["风衣", "高领毛衣", "直筒裤", "短靴"], note: "经典秋日搭配" },
      { items: ["卫衣", "百褶裙", "马丁靴"], note: "甜酷风，温度和风度都有" },
      { items: ["西装外套", "T恤", "高腰裤", "乐福鞋"], note: "知性帅气" },
    ],
    winter: [
      { items: ["大衣", "毛衣", "加绒裤", "靴子"], note: "保暖第一，大衣气场满分" },
      { items: ["羽绒服", "卫衣", "加绒裤", "雪地靴"], note: "冷到不行就这样穿" },
      { items: ["羊羔绒外套", "高领", "半裙", "加绒靴"], note: "软糯可爱" },
    ],
  },

  getSeason() {
    const m = new Date().getMonth() + 1;
    if (m >= 3 && m <= 5) return 'spring';
    if (m >= 6 && m <= 8) return 'summer';
    if (m >= 9 && m <= 11) return 'autumn';
    return 'winter';
  },

  seasonLabels: { spring: '春', summer: '夏', autumn: '秋', winter: '冬' },

  // Community stories (read-only feed)
  defaultStories: [
    { id: "s1", category: "voice", date: "2026-01-15", likes: 34,
      content: "练了三个月声音，最大感悟是：共鸣比音高重要得多。一开始我拼命提高音调，听起来很假很累。后来专注于把共鸣从胸腔移到头部，即使音高没怎么变，声音听起来就完全不同了。" },
    { id: "s2", category: "makeup", date: "2026-02-08", likes: 56,
      content: "分享遮盖胡茬青影心得：橘色遮瑕中和是关键！NYX橘色遮瑕膏薄薄一层拍在青影区域，等30秒再上肤色遮瑕，然后散粉定妆。出门一整天都不会透出青影。" },
    { id: "s3", category: "fashion", date: "2026-02-20", likes: 42,
      content: "刚开始穿女装出门建议从安全搭配开始：高腰阔腿裤+稍微宽松的上衣，既有腰线又不会太紧贴身体。第一次穿裙子出门约了最信任的朋友一起。" },
    { id: "s4", category: "exercise", date: "2026-03-05", likes: 29,
      content: "坚持臀桥和深蹲3个月臀部真的有变化！每天15分钟，不用器械在床上就能做。关键是坚持。" },
    { id: "s5", category: "skin", date: "2026-03-18", likes: 38,
      content: "防晒是最有效的抗衰老手段。每天出门前必涂SPF50，即使阴天。两个月后感觉肤色均匀了很多。" },
    { id: "s6", category: "general", date: "2026-04-01", likes: 67,
      content: "今天点咖啡的时候，店员用了我想要的称呼。虽然是一件很小的事，但那种被正确对待的感觉真的很暖。" },
  ],

  storyCategories: {
    voice: "声音", skin: "护肤", makeup: "化妆",
    fashion: "穿搭", exercise: "锻炼", general: "日常",
  },
};
