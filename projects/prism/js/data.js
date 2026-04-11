/* Prism Data v0.3.0 */
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
    "进步不需要别人看到，我自己知道就好。",
    "每照一次镜子，试着找到一个喜欢的地方。",
    "我的独特不是缺陷，是特征。",
    "改变是一个过程，我有耐心。",
    "今天的练习是为明天的自信投资。",
    "我为自己每天的努力感到骄傲。",
    "身体的变化需要时间，但每一步都算数。",
    "我的价值不取决于别人的眼光。",
    "每一次练习都在重塑我的可能性。",
    "我值得拥有让自己舒服的身体。",
    "探索自我的路上，不需要和任何人比较。",
    "今天也要善待这副陪伴我一生的身体。"
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
    "明天见啦，路上小心哦。"
  ],

  skinRoutines: {
    morning: [
      { step: "洁面", tip: "用温水和温和洁面乳，不要过度清洁", seconds: 120 },
      { step: "化妆水/爽肤水", tip: "用化妆棉或手轻拍，平衡肌肤pH", seconds: 60 },
      { step: "精华液", tip: "维C精华在早上用，抗氧化", seconds: 60 },
      { step: "眼霜", tip: "无名指轻拍眼周，力度要轻", seconds: 60 },
      { step: "面霜/乳液", tip: "根据肤质选择质地", seconds: 60 },
      { step: "防晒", tip: "SPF30+，用量要足（一元硬币大小）", seconds: 60 }
    ],
    evening: [
      { step: "卸妆", tip: "先卸妆再洁面，即使只涂了防晒", seconds: 120 },
      { step: "洁面", tip: "二次清洁，确保干净", seconds: 120 },
      { step: "化妆水", tip: "补水打底", seconds: 60 },
      { step: "精华液", tip: "烟酰胺/视黄醇等活性成分晚上用", seconds: 60 },
      { step: "眼霜", tip: "夜间修复眼周肌肤", seconds: 60 },
      { step: "面霜", tip: "晚间可以用更滋润的质地", seconds: 60 }
    ]
  },

  workouts: {
    full: {
      name: "全身塑形", duration: 25,
      exercises: [
        { name: "深蹲", reps: "15次×3组", desc: "双脚与肩同宽，臀部向后坐", icon: "🦵" },
        { name: "臀桥", reps: "20次×3组", desc: "仰卧屈膝，发力抬起臀部", icon: "🍑" },
        { name: "平板支撑", reps: "30秒×3组", desc: "身体一条直线，核心收紧", icon: "💪" },
        { name: "侧抬腿", reps: "每侧15次×3组", desc: "侧卧上方腿伸直抬起", icon: "🦿" },
        { name: "俯身划船", reps: "12次×3组", desc: "哑铃或水瓶，收紧背部", icon: "🏋️" },
        { name: "猫牛式", reps: "10次", desc: "四点跪姿交替弓背塌腰", icon: "🐱" }
      ]
    },
    lower: {
      name: "臀腿专项", duration: 30,
      exercises: [
        { name: "深蹲", reps: "15次×4组", desc: "注意臀部发力感", icon: "🦵" },
        { name: "臀桥", reps: "20次×4组", desc: "可以单腿增加难度", icon: "🍑" },
        { name: "保加利亚分腿蹲", reps: "每侧12次×3组", desc: "后脚放椅子上", icon: "🏃" },
        { name: "侧卧蚌式", reps: "每侧20次×3组", desc: "侧卧屈膝，上方膝盖打开", icon: "🐚" },
        { name: "消防栓式", reps: "每侧15次×3组", desc: "四点跪姿侧上方抬起", icon: "🦿" },
        { name: "小腿提踵", reps: "20次×3组", desc: "站立脚跟抬起放下", icon: "🦶" }
      ]
    },
    core: {
      name: "腰腹核心", duration: 20,
      exercises: [
        { name: "平板支撑", reps: "45秒×3组", desc: "核心收紧，不塌腰", icon: "💪" },
        { name: "死虫式", reps: "每侧10次×3组", desc: "仰卧对侧手脚交替", icon: "🐛" },
        { name: "卷腹", reps: "15次×3组", desc: "腹部发力，不用脖子", icon: "🔄" },
        { name: "俄罗斯转体", reps: "20次×3组", desc: "坐姿微后仰左右转", icon: "🌀" },
        { name: "登山者", reps: "30秒×3组", desc: "平板撑交替提膝", icon: "⛰️" },
        { name: "仰卧骑车", reps: "20次×3组", desc: "仰卧双脚模拟骑车", icon: "🚴" }
      ]
    },
    upper: {
      name: "上身柔化", duration: 20,
      exercises: [
        { name: "跪姿俯卧撑", reps: "10次×3组", desc: "膝盖着地降低难度", icon: "💪" },
        { name: "弹力带拉伸", reps: "15次×3组", desc: "开肩展背", icon: "🎗️" },
        { name: "超人式", reps: "12次×3组", desc: "俯卧同时抬起手脚", icon: "🦸" },
        { name: "手臂画圈", reps: "30秒×3组", desc: "双臂伸直画小圈", icon: "⭕" },
        { name: "墙壁推", reps: "15次×3组", desc: "面对墙壁推墙", icon: "🧱" },
        { name: "肩部拉伸", reps: "每侧30秒", desc: "一手横过胸前辅助拉伸", icon: "🙆" }
      ]
    },
    yoga: {
      name: "柔韧瑜伽", duration: 25,
      exercises: [
        { name: "山式站立", reps: "1分钟", desc: "双脚并拢感受平衡", icon: "🧘" },
        { name: "战士一式", reps: "每侧30秒", desc: "前弓步双手上举", icon: "⚔️" },
        { name: "战士二式", reps: "每侧30秒", desc: "侧弓步双臂侧平举", icon: "🗡️" },
        { name: "三角式", reps: "每侧30秒", desc: "侧弯身一手触脚踝", icon: "📐" },
        { name: "树式", reps: "每侧30秒", desc: "单脚站立一脚抵大腿", icon: "🌳" },
        { name: "下犬式", reps: "5次呼吸", desc: "倒V字形", icon: "🐕" },
        { name: "桥式", reps: "5次呼吸", desc: "仰卧屈膝抬起臀部", icon: "🌉" },
        { name: "婴儿式", reps: "1分钟", desc: "跪坐放松", icon: "👶" }
      ]
    }
  },

  // Weekly training schedule
  weeklyPlan: {
    1: { type: 'voice',  label: '声音训练日', icon: '🎤' },
    2: { type: 'body',   label: '身体锻炼日', icon: '💪' },
    3: { type: 'voice',  label: '声音训练日', icon: '🎤' },
    4: { type: 'body',   label: '身体锻炼日', icon: '💪' },
    5: { type: 'voice',  label: '声音训练日', icon: '🎤' },
    6: { type: 'body',   label: '身体锻炼日', icon: '💪' },
    0: { type: 'rest',   label: '休息与回顾', icon: '🌸' }
  },

  selfCareItems: [
    "喝了足够的水（8杯以上）",
    "完成护肤流程",
    "至少30分钟运动/拉伸",
    "练习了声音训练",
    "吃了营养均衡的三餐",
    "照镜子时对自己说了一句好话",
    "做了让自己开心的事",
    "早点上床休息"
  ],

  defaultStories: [
    {
      id: "s1", category: "voice", anon: true, likes: 34,
      content: "练了三个月声音，最大感悟是：共鸣比音高重要得多。一开始我拼命提高音调，听起来很假很累。后来专注于把共鸣从胸腔移到头部，即使音高没怎么变，声音听起来就完全不同了。",
      date: "2026-01-15"
    },
    {
      id: "s2", category: "makeup", anon: true, likes: 56,
      content: "分享我的遮盖胡茬青影心得：橘色遮瑕中和是关键！NYX橘色遮瑕膏薄薄一层拍在青影区域，等30秒再上肤色遮瑕，然后散粉定妆。出门一整天都不会透出青影。",
      date: "2026-02-08"
    },
    {
      id: "s3", category: "fashion", anon: true, likes: 42,
      content: "刚开始穿女装出门的时候超级紧张。建议从安全搭配开始：高腰阔腿裤+稍微宽松的上衣，既有腰线又不会太紧贴身体。第一次穿裙子出门约了最信任的朋友一起。",
      date: "2026-02-20"
    },
    {
      id: "s4", category: "exercise", anon: true, likes: 29,
      content: "坚持臀桥和深蹲3个月臀部真的有变化！每天15分钟，不用器械在床上就能做。关键是坚持。",
      date: "2026-03-05"
    },
    {
      id: "s5", category: "skin", anon: true, likes: 38,
      content: "之前一直忽略防晒，后来才知道防晒是最有效的抗衰老手段。每天出门前必涂SPF50，即使阴天。两个月后感觉肤色均匀了很多。",
      date: "2026-03-18"
    },
    {
      id: "s6", category: "general", anon: true, likes: 67,
      content: "今天点咖啡的时候，店员用了我想要的称呼。虽然是一件很小的事，但那种被正确对待的感觉真的很暖。",
      date: "2026-04-01"
    }
  ],

  storyCategories: {
    voice: "声音训练", skin: "护肤经验", makeup: "化妆技巧",
    fashion: "穿搭心得", exercise: "锻炼经验", general: "日常分享"
  },

  productTypes: {
    cleanser: "洁面", toner: "化妆水", serum: "精华",
    moisturizer: "面霜", sunscreen: "防晒", mask: "面膜",
    eye: "眼霜", other: "其他"
  },

  // Outfit suggestions by weather/temp range
  outfitSuggestions: {
    hot:    { label: "炎热", items: ["吊带/背心", "短裤/短裙", "凉鞋", "防晒衣备着"] },
    warm:   { label: "温暖", items: ["T恤/衬衫", "阔腿裤/半裙", "薄开衫", "帆布鞋"] },
    cool:   { label: "凉爽", items: ["长袖衬衫", "长裤/长裙", "薄外套", "乐福鞋"] },
    cold:   { label: "寒冷", items: ["毛衣/卫衣", "厚裤/加绒裤", "大衣/羽绒服", "靴子"] }
  }
};
