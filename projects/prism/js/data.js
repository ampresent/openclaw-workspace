/* Prism Data - Body-focused content */
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

  // Voice training practice sentences
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

  // Skin care routines
  skinRoutines: {
    morning: [
      { step: "洁面", tip: "用温水和温和洁面乳，不要过度清洁" },
      { step: "化妆水/爽肤水", tip: "用化妆棉或手轻拍，平衡肌肤pH" },
      { step: "精华液", tip: "维C精华在早上用，抗氧化" },
      { step: "眼霜", tip: "无名指轻拍眼周，力度要轻" },
      { step: "面霜/乳液", tip: "根据肤质选择质地" },
      { step: "防晒", tip: "SPF30+，用量要足（一元硬币大小）" }
    ],
    evening: [
      { step: "卸妆", tip: "先卸妆再洁面，即使只涂了防晒" },
      { step: "洁面", tip: "二次清洁，确保干净" },
      { step: "化妆水", tip: "补水打底" },
      { step: "精华液", tip: "烟酰胺/视黄醇等活性成分晚上用" },
      { step: "眼霜", tip: "夜间修复眼周肌肤" },
      { step: "面霜", tip: "晚间可以用更滋润的质地" }
    ]
  },

  // Workout plans
  workouts: {
    full: [
      { name: "深蹲", reps: "15次×3组", desc: "双脚与肩同宽，臀部向后坐，膝盖不超过脚尖" },
      { name: "臀桥", reps: "20次×3组", desc: "仰卧屈膝，发力抬起臀部，顶部挤压臀肌" },
      { name: "平板支撑", reps: "30秒×3组", desc: "身体一条直线，核心收紧" },
      { name: "侧抬腿", reps: "每侧15次×3组", desc: "侧卧，上方腿伸直抬起，锻炼臀中肌" },
      { name: "俯身划船", reps: "12次×3组", desc: "哑铃或水瓶，收紧背部" },
      { name: "猫牛式", reps: "10次", desc: "四点跪姿交替弓背和塌腰" }
    ],
    lower: [
      { name: "深蹲", reps: "15次×4组", desc: "注意臀部发力感" },
      { name: "臀桥", reps: "20次×4组", desc: "可以单腿增加难度" },
      { name: "保加利亚分腿蹲", reps: "每侧12次×3组", desc: "后脚放椅子上，前腿发力" },
      { name: "侧卧蚌式", reps: "每侧20次×3组", desc: "侧卧屈膝，上方膝盖打开" },
      { name: "消防栓式", reps: "每侧15次×3组", desc: "四点跪姿，膝盖向侧上方抬起" },
      { name: "小腿提踵", reps: "20次×3组", desc: "站立，脚跟抬起再放下" }
    ],
    core: [
      { name: "平板支撑", reps: "45秒×3组", desc: "核心收紧，身体不塌腰" },
      { name: "死虫式", reps: "每侧10次×3组", desc: "仰卧，对侧手脚交替伸展" },
      { name: "卷腹", reps: "15次×3组", desc: "腹部发力卷起，不要用脖子" },
      { name: "俄罗斯转体", reps: "20次×3组", desc: "坐姿微后仰，双手左右转体" },
      { name: "登山者", reps: "30秒×3组", desc: "平板撑姿势交替提膝" },
      { name: "仰卧骑车", reps: "20次×3组", desc: "仰卧双脚模拟骑车动作" }
    ],
    upper: [
      { name: "跪姿俯卧撑", reps: "10次×3组", desc: "膝盖着地，降低难度" },
      { name: "弹力带拉伸", reps: "15次×3组", desc: "开肩展背，轻阻力" },
      { name: "超人式", reps: "12次×3组", desc: "俯卧，同时抬起手脚" },
      { name: "手臂画圈", reps: "30秒×3组", desc: "双臂伸直画小圈，轻重量" },
      { name: "墙壁推", reps: "15次×3组", desc: "面对墙壁做推墙动作" },
      { name: "肩部拉伸", reps: "每侧30秒", desc: "一手横过胸前，另一手辅助拉伸" }
    ],
    yoga: [
      { name: "山式站立", reps: "1分钟", desc: "双脚并拢，感受重心平衡" },
      { name: "战士一式", reps: "每侧30秒", desc: "前弓步，后腿伸直，双手上举" },
      { name: "战士二式", reps: "每侧30秒", desc: "侧弓步，双臂侧平举" },
      { name: "三角式", reps: "每侧30秒", desc: "侧弯身，一手触脚踝" },
      { name: "树式", reps: "每侧30秒", desc: "单脚站立，一脚抵大腿内侧" },
      { name: "下犬式", reps: "保持5次呼吸", desc: "倒V字形，手掌脚掌着地" },
      { name: "桥式", reps: "保持5次呼吸", desc: "仰卧屈膝抬起臀部" },
      { name: "婴儿式", reps: "保持1分钟", desc: "跪坐，上身前倾放松" }
    ]
  },

  // Self-care items for wellness
  selfCareItems: [
    "今天喝了足够的水（8杯以上）",
    "做了护肤流程",
    "有至少30分钟的运动/拉伸",
    "练习了声音训练",
    "吃了营养均衡的三餐",
    "照镜子时对自己说了一句好话",
    "做了让自己开心的事",
    "早点上床休息",
    "整理了仪表/穿搭",
    "和朋友聊了天"
  ],

  // Default stories
  defaultStories: [
    {
      id: "s1", category: "voice", anon: true, likes: 34,
      content: "练了三个月声音，最大感悟是：共鸣比音高重要得多。一开始我拼命提高音调，听起来很假很累。后来专注于把共鸣从胸腔移到头部，即使音高没怎么变，声音听起来就完全不同了。给还在挣扎的姐妹：不要急着追求高音，先把共鸣练好。",
      date: "2026-01-15"
    },
    {
      id: "s2", category: "makeup", anon: true, likes: 56,
      content: "分享我的遮盖胡茬青影心得：橘色遮瑕中和是关键！我用的是NYX的橘色遮瑕膏，薄薄一层拍在青影区域，等30秒再上肤色遮瑕。然后散粉定妆。现在出门一整天都不会透出青影。",
      date: "2026-02-08"
    },
    {
      id: "s3", category: "fashion", anon: true, likes: 42,
      content: "刚开始穿女装出门的时候超级紧张。我的建议是先从安全的搭配开始：高腰阔腿裤+稍微宽松的上衣，既有腰线又不会太紧贴身体。等习惯了再慢慢尝试更贴身的款式。第一次穿裙子出门的时候，我约了最信任的朋友一起，有伴真的安心很多。",
      date: "2026-02-20"
    },
    {
      id: "s4", category: "exercise", anon: true, likes: 29,
      content: "坚持做臀桥和深蹲3个月，臀部真的有变化！拍照对比就能看出来。我的routine：每天15分钟，臀桥20个×4组 + 深蹲15个×3组 + 侧抬腿每侧15个×3组。不用器械，在床上就能做。关键是坚持，不用每次都练很久。",
      date: "2026-03-05"
    },
    {
      id: "s5", category: "skin", anon: true, likes: 38,
      content: "之前一直忽略防晒，后来才知道防晒是最有效的抗衰老手段。现在每天出门前必涂SPF50，即使阴天。两个月后感觉肤色均匀了很多，之前的暗沉改善明显。姐妹们一定要重视防晒！",
      date: "2026-03-18"
    },
    {
      id: "s6", category: "general", anon: true, likes: 67,
      content: "今天点咖啡的时候，店员用了我想要的称呼。虽然是一件很小的事，但那种被正确对待的感觉真的很暖。改变需要时间，但每一个这样的小瞬间都值得记住。",
      date: "2026-04-01"
    }
  ],

  // Resources
  resources: {
    voice: [
      { title: "TransVoiceLessons (YouTube)", desc: "系统讲解共鸣、音高、语调的训练频道" },
      { title: "r/transvoice (Reddit)", desc: "可以分享录音获取反馈的社区" },
      { title: "Vocal Pitch Monitor App", desc: "实时显示音高的练习辅助工具" }
    ],
    skin: [
      { title: "r/SkincareAddiction", desc: "最大的护肤社区，产品推荐和知识科普" },
      { title: "Lab Muffin Beauty Science", desc: "化学博士的护肤成分科普" }
    ],
    makeup: [
      { title: "Wayne Goss (YouTube)", desc: "专业化妆师，简洁实用的教程" },
      { title: "NikkieTutorials", desc: "跨性别美妆博主，详细技巧教程" }
    ],
    exercise: [
      { title: "Yoga With Adriene", desc: "适合所有水平的瑜伽教程" },
      { title: "Blogilates", desc: "女性化塑形居家运动" }
    ]
  },

  storyCategories: {
    voice: "声音训练", skin: "护肤经验", makeup: "化妆技巧",
    fashion: "穿搭心得", exercise: "锻炼经验", general: "日常分享"
  },

  productTypes: {
    cleanser: "洁面", toner: "化妆水", serum: "精华",
    moisturizer: "面霜", sunscreen: "防晒", mask: "面膜",
    eye: "眼霜", other: "其他"
  }
};
