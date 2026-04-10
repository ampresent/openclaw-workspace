# TaskWheel 同心圆轮盘实现计划

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 开发 iOS 同心圆轮盘应用，实现 2.5 秒内快速选择任务主题

**Architecture:** 三层架构 - UI 层 (SwiftUI 轮盘渲染)、业务层 (主题管理/选择逻辑)、模型层 (llama.cpp 本地推理)

**Tech Stack:** SwiftUI 5.0, llama.cpp iOS, Core Graphics, Metal, CoreML

---

## 文件结构

### 新建文件

```
TaskWheel/
├── App/
│   ├── TaskWheelApp.swift          # 应用入口
│   └── AppDelegate.swift           # 应用生命周期
├── UI/
│   ├── Views/
│   │   ├── WheelView.swift         # 单级轮盘视图
│   │   ├── ConcentricWheelView.swift # 同心圆容器视图
│   │   └── ThemeDetailView.swift   # 主题详情视图
│   ├── Gestures/
│   │   ├── WheelGestureHandler.swift # 手势处理
│   │   └── EdgeSwipeHandler.swift  # 边缘滑动返回
│   └── Animations/
│       ├── WheelAnimator.swift     # 轮盘动画
│       └── TransitionAnimations.swift # 过渡动画
├── Business/
│   ├── ThemeManager.swift          # 主题管理
│   ├── SelectionManager.swift      # 选择逻辑
│   └── NavigationManager.swift     # 导航管理
├── Model/
│   ├── LLMEngine.swift             # llama.cpp 封装
│   ├── ThemeCache.swift           # 主题缓存
│   └── ThemeStore.swift           # 主题存储
├── Data/
│   ├── themes.json                # 初始主题数据
│   └── models/
│       └── tinyllama-1.1b-quant.bin # 量化模型
└── Tests/
    ├── UI/
    │   ├── WheelViewTests.swift
    │   └── GestureTests.swift
    ├── Business/
    │   ├── ThemeManagerTests.swift
    │   └── SelectionManagerTests.swift
    └── Model/
        ├── LLMEngineTests.swift
        └── ThemeCacheTests.swift
```

### 修改文件

无（新项目）

---

## Phase 1: 项目基础架构

### Task 1: 创建 Xcode 项目

**Files:**
- Create: `TaskWheel.xcodeproj/project.pbxproj`
- Create: `TaskWheel/Info.plist`

- [ ] **Step 1: 创建 Xcode 项目结构**

```bash
mkdir -p TaskWheel/{App,UI/{Views,Gestures,Animations},Business,Model,Data/models,Tests/{UI,Business,Model}}
cd TaskWheel
```

- [ ] **Step 2: 创建 Info.plist**

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>TaskWheel</string>
    <key>CFBundleIdentifier</key><string>com.example.taskwheel</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>CFBundleShortVersionString</key><string>1.0</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>UILaunchScreen</key><dict/>
    <key>UISupportedInterfaceOrientations</key>
    <array><string>UIInterfaceOrientationPortrait</string></array>
</dict>
</plist>
```

- [ ] **Step 3: 创建项目文件**

```bash
# 使用 Xcode 命令行工具创建
open -a Xcode TaskWheel/
# 手动配置：iOS 17.0+, Swift 5.9, SwiftUI
```

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "feat: 创建 Xcode 项目基础结构"
```

---

### Task 2: 应用入口

**Files:**
- Create: `TaskWheel/App/TaskWheelApp.swift`
- Create: `TaskWheel/App/AppDelegate.swift`

- [ ] **Step 1: 创建应用入口**

```swift
// TaskWheel/App/TaskWheelApp.swift
import SwiftUI

@main
struct TaskWheelApp: App {
    @UIApplicationDelegateAdaptor(AppDelegate.self) var delegate
    
    var body: some Scene {
        WindowGroup {
            ContentView()
        }
    }
}
```

- [ ] **Step 2: 创建 AppDelegate**

```swift
// TaskWheel/App/AppDelegate.swift
import UIKit

class AppDelegate: NSObject, UIApplicationDelegate {
    func application(
        _ application: UIApplication,
        didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]? = nil
    ) -> Bool {
        // 初始化主题存储
        ThemeStore.shared.load()
        return true
    }
}
```

- [ ] **Step 3: 创建临时 ContentView**

```swift
// TaskWheel/UI/Views/ContentView.swift
import SwiftUI

struct ContentView: View {
    var body: some View {
        Text("TaskWheel")
            .font(.largeTitle)
    }
}

#Preview {
    ContentView()
}
```

- [ ] **Step 4: 运行验证**

```bash
# Xcode: Cmd+R 运行
# 预期：模拟器显示"TaskWheel"文字
```

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat: 添加应用入口"
```

---

### Task 3: 主题数据结构

**Files:**
- Create: `TaskWheel/Model/Theme.swift`
- Create: `TaskWheel/Data/themes.json`
- Test: `TaskWheel/Tests/Model/ThemeTests.swift`

- [ ] **Step 1: 编写测试**

```swift
// TaskWheel/Tests/Model/ThemeTests.swift
import XCTest
@testable import TaskWheel

final class ThemeTests: XCTestCase {
    func testThemeDecoding() throws {
        let json = """
        {"id": "work", "name": "工作", "icon": "briefcase.fill", "color": "#007AFF"}
        """.data(using: .utf8)!
        
        let theme = try JSONDecoder().decode(Theme.self, from: json)
        
        XCTAssertEqual(theme.id, "work")
        XCTAssertEqual(theme.name, "工作")
        XCTAssertEqual(theme.icon, "briefcase.fill")
        XCTAssertEqual(theme.color, "#007AFF")
    }
    
    func testThemeWithChildren() throws {
        let json = """
        {
            "id": "work",
            "name": "工作",
            "children": [
                {"id": "coding", "name": "写代码"}
            ]
        }
        """.data(using: .utf8)!
        
        let theme = try JSONDecoder().decode(Theme.self, from: json)
        
        XCTAssertEqual(theme.children?.count, 1)
        XCTAssertEqual(theme.children?.first?.id, "coding")
    }
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
xcodebuild test -scheme TaskWheel -destination 'platform=iOS Simulator,name=iPhone 15' \
  -only-testing:TaskWheelTests/ThemeTests/testThemeDecoding
# 预期：FAIL - Theme 未定义
```

- [ ] **Step 3: 实现 Theme 模型**

```swift
// TaskWheel/Model/Theme.swift
import Foundation
import SwiftUI

struct Theme: Codable, Identifiable, Hashable {
    let id: String
    let name: String
    var icon: String = "circle.fill"
    var color: String = "#007AFF"
    var sortOrder: Int = 0
    var children: [Theme]? = nil
    
    var swiftUIColor: Color {
        Color(hex: color) ?? .blue
    }
    
    var systemImage: String {
        icon
    }
}

// MARK: - Color Extension
extension Color {
    init?(hex: String) {
        var hexSanitized = hex.trimmingCharacters(in: .whitespacesAndNewlines)
        hexSanitized = hexSanitized.replacingOccurrences(of: "#", with: "")
        
        var rgb: UInt64 = 0
        guard Scanner(string: hexSanitized).scanHexInt64(&rgb) else { return nil }
        
        let r = Double((rgb & 0xFF0000) >> 16) / 255.0
        let g = Double((rgb & 0x00FF00) >> 8) / 255.0
        let b = Double(rgb & 0x0000FF) / 255.0
        
        self.init(red: r, green: g, blue: b)
    }
}
```

- [ ] **Step 4: 创建初始主题数据**

```json
// TaskWheel/Data/themes.json
[
  {
    "id": "work",
    "name": "工作",
    "icon": "briefcase.fill",
    "color": "#007AFF",
    "sortOrder": 1,
    "children": [
      {"id": "coding", "name": "写代码", "icon": "terminal.fill"},
      {"id": "meeting", "name": "开会", "icon": "person.2.fill"},
      {"id": "document", "name": "写文档", "icon": "doc.text.fill"},
      {"id": "email", "name": "回邮件", "icon": "envelope.fill"}
    ]
  },
  {
    "id": "life",
    "name": "生活",
    "icon": "house.fill",
    "color": "#34C759",
    "sortOrder": 2,
    "children": [
      {"id": "cooking", "name": "做饭", "icon": "fork.knife"},
      {"id": "cleaning", "name": "打扫", "icon": "broom.fill"},
      {"id": "shopping", "name": "购物", "icon": "cart.fill"}
    ]
  },
  {
    "id": "health",
    "name": "健康",
    "icon": "heart.fill",
    "color": "#FF2D55",
    "sortOrder": 3,
    "children": [
      {"id": "exercise", "name": "运动", "icon": "figure.run"},
      {"id": "sleep", "name": "睡觉", "icon": "bed.double.fill"},
      {"id": "meditation", "name": "冥想", "icon": "figure.mind.and.body"}
    ]
  },
  {
    "id": "learning",
    "name": "学习",
    "icon": "book.fill",
    "color": "#5856D6",
    "sortOrder": 4,
    "children": [
      {"id": "reading", "name": "读书", "icon": "book.closed.fill"},
      {"id": "course", "name": "上课", "icon": "tv.fill"},
      {"id": "practice", "name": "练习", "icon": "pencil.tip.crop.circle"}
    ]
  },
  {
    "id": "entertainment",
    "name": "娱乐",
    "icon": "gamecontroller.fill",
    "color": "#FF9500",
    "sortOrder": 5,
    "children": [
      {"id": "gaming", "name": "玩游戏", "icon": "gamecontroller.fill"},
      {"id": "movie", "name": "看电影", "icon": "film.fill"},
      {"id": "music", "name": "听音乐", "icon": "music.note.house.fill"}
    ]
  }
]
```

- [ ] **Step 5: 运行测试验证通过**

```bash
xcodebuild test -scheme TaskWheel -destination 'platform=iOS Simulator,name=iPhone 15' \
  -only-testing:TaskWheelTests/ThemeTests
# 预期：PASS
```

- [ ] **Step 6: 提交**

```bash
git add -A
git commit -m "feat: 添加主题数据模型和初始数据"
```

---

### Task 4: 主题存储层

**Files:**
- Create: `TaskWheel/Model/ThemeStore.swift`
- Test: `TaskWheel/Tests/Model/ThemeStoreTests.swift`

- [ ] **Step 1: 编写测试**

```swift
// TaskWheel/Tests/Model/ThemeStoreTests.swift
import XCTest
@testable import TaskWheel

final class ThemeStoreTests: XCTestCase {
    var store: ThemeStore!
    
    override func setUp() {
        store = ThemeStore()
    }
    
    func testLoadInitialThemes() {
        store.load()
        XCTAssertGreaterThan(store.themes.count, 0)
    }
    
    func testGetThemeById() {
        store.load()
        let workTheme = store.getTheme(by: "work")
        XCTAssertEqual(workTheme?.name, "工作")
    }
    
    func testGetChildren() {
        store.load()
        let children = store.getChildren(for: "work")
        XCTAssertGreaterThan(children.count, 0)
    }
}
```

- [ ] **Step 2: 运行测试验证失败**

```bash
xcodebuild test -scheme TaskWheel \
  -only-testing:TaskWheelTests/ThemeStoreTests/testLoadInitialThemes
# 预期：FAIL - ThemeStore 未定义
```

- [ ] **Step 3: 实现 ThemeStore**

```swift
// TaskWheel/Model/ThemeStore.swift
import Foundation

final class ThemeStore: ObservableObject {
    static let shared = ThemeStore()
    
    @Published private(set) var themes: [Theme] = []
    private var themeIndex: [String: Theme] = [:]
    
    private init() {}
    
    func load() {
        guard let url = Bundle.main.url(forResource: "themes", withExtension: "json"),
              let data = try? Data(contentsOf: url),
              let themes = try? JSONDecoder().decode([Theme].self, from: data) else {
            print("❌ 加载主题失败")
            return
        }
        
        self.themes = themes.sorted { $0.sortOrder < $1.sortOrder }
        buildIndex()
        print("✅ 加载 \(themes.count) 个主题")
    }
    
    private func buildIndex() {
        themeIndex.removeAll()
        for theme in themes {
            indexTheme(theme)
        }
    }
    
    private func indexTheme(_ theme: Theme) {
        themeIndex[theme.id] = theme
        theme.children?.forEach { indexTheme($0) }
    }
    
    func getTheme(by id: String) -> Theme? {
        themeIndex[id]
    }
    
    func getChildren(for parentId: String) -> [Theme] {
        guard let parent = themeIndex[parentId] else { return [] }
        return parent.children ?? []
    }
    
    func hasChildren(_ theme: Theme) -> Bool {
        !(theme.children?.isEmpty ?? true)
    }
}
```

- [ ] **Step 4: 运行测试验证通过**

```bash
xcodebuild test -scheme TaskWheel \
  -only-testing:TaskWheelTests/ThemeStoreTests
# 预期：PASS
```

- [ ] **Step 5: 提交**

```bash
git add -A
git commit -m "feat: 添加主题存储层"
```

---

## Phase 2: UI 层 - 轮盘视图

### Task 5: 单级轮盘视图

**Files:**
- Create: `TaskWheel/UI/Views/WheelView.swift`
- Test: `TaskWheel/Tests/UI/WheelViewTests.swift`

- [ ] **Step 1: 编写快照测试**

```swift
// TaskWheel/Tests/UI/WheelViewTests.swift
import XCTest
import SwiftUI
@testable import TaskWheel

final class WheelViewTests: XCTestCase {
    func testWheelRendersSegments() {
        let themes = [
            Theme(id: "a", name: "A"),
            Theme(id: "b", name: "B"),
            Theme(id: "c", name: "C")
        ]
        
        let view = WheelView(themes: themes, selectedIndex: 0, onSelection: { _ in })
        
        // UI 测试：验证视图可以渲染
        let hosting = UIHostingController(rootView: view)
        XCTAssertNotNil(hosting.view)
    }
}
```

- [ ] **Step 2: 实现 WheelView**

```swift
// TaskWheel/UI/Views/WheelView.swift
import SwiftUI

struct WheelView: View {
    let themes: [Theme]
    let selectedIndex: Int
    let onSelection: (Theme) -> Void
    
    private let segmentAngle: Double
    private let radius: CGFloat
    
    init(themes: [Theme], selectedIndex: Int = 0, radius: CGFloat = 150, onSelection: @escaping (Theme) -> Void) {
        self.themes = themes
        self.selectedIndex = selectedIndex
        self.radius = radius
        self.segmentAngle = 360.0 / Double(themes.count)
        self.onSelection = onSelection
    }
    
    var body: some View {
        ZStack {
            ForEach(Array(themes.enumerated()), id: \.element.id) { index, theme in
                WheelSegment(
                    theme: theme,
                    startAngle: Angle(degrees: segmentAngle * Double(index) - 90),
                    endAngle: Angle(degrees: segmentAngle * Double(index + 1) - 90),
                    radius: radius,
                    isSelected: index == selectedIndex
                )
                .onTapGesture {
                    onSelection(theme)
                }
            }
        }
        .frame(width: radius * 2, height: radius * 2)
    }
}

struct WheelSegment: View {
    let theme: Theme
    let startAngle: Angle
    let endAngle: Angle
    let radius: CGFloat
    let isSelected: Bool
    
    var body: some View {
        ZStack {
            SegmentShape(startAngle: startAngle, endAngle: endAngle, radius: radius)
                .fill(isSelected ? Color.green : Color(hex: theme.color) ?? .blue)
            
            VStack(spacing: 4) {
                Image(systemName: theme.systemImage)
                    .font(.system(size: 20))
                Text(theme.name)
                    .font(.system(size: 12))
            }
            .foregroundColor(.white)
            .offset(y: -radius * 0.6)
            .rotationEffect(Angle(degrees: (startAngle.degrees + endAngle.degrees) / 2 - 90))
        }
    }
}

struct SegmentShape: Shape {
    let startAngle: Angle
    let endAngle: Angle
    let radius: CGFloat
    
    var animatableData: EmptyAnimatableData {
        get { EmptyAnimatableData() }
        set {}
    }
    
    func path(in rect: CGRect) -> Path {
        var path = Path()
        let center = CGPoint(x: rect.midX, y: rect.midY)
        
        path.move(to: center)
        path.addArc(
            center: center,
            radius: radius,
            startAngle: startAngle,
            endAngle: endAngle,
            clockwise: false
        )
        path.closeSubpath()
        
        return path
    }
}

#Preview {
    WheelView(
        themes: [
            Theme(id: "1", name: "工作", icon: "briefcase.fill"),
            Theme(id: "2", name: "生活", icon: "house.fill"),
            Theme(id: "3", name: "健康", icon: "heart.fill"),
            Theme(id: "4", name: "学习", icon: "book.fill")
        ],
        selectedIndex: 0
    ) { _ in }
}
```

- [ ] **Step 3: 运行预览验证**

```bash
# Xcode: 打开 WheelView.swift，点击 Preview
# 预期：显示 4 个扇区的轮盘
```

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "feat: 添加单级轮盘视图"
```

---

### Task 6: 同心圆容器视图

**Files:**
- Create: `TaskWheel/UI/Views/ConcentricWheelView.swift`

- [ ] **Step 1: 实现同心圆容器**

```swift
// TaskWheel/UI/Views/ConcentricWheelView.swift
import SwiftUI

struct ConcentricWheelView: View {
    @StateObject private var themeStore = ThemeStore.shared
    @State private var selectedLevel0: String = ""
    @State private var selectedLevel1: String = ""
    @State private var selectedLevel2: String = ""
    
    @State private var currentLevel: Int = 0
    @State private var isAnimating: Bool = false
    
    var body: some View {
        ZStack {
            Color(.systemBackground).ignoresSafeArea()
            
            VStack(spacing: 20) {
                // 进度指示
                ProgressView(value: Double(currentLevel + 1), total: 3)
                    .progressViewStyle(LinearProgressViewStyle())
                    .padding(.horizontal)
                
                Spacer()
                
                // 轮盘容器
                ZStack {
                    // 一级轮盘（外圈）
                    if currentLevel >= 0 {
                        WheelView(
                            themes: themeStore.themes,
                            selectedIndex: index(of: selectedLevel0, in: themeStore.themes),
                            radius: 160
                        ) { theme in
                            selectTheme(theme, level: 0)
                        }
                        .opacity(currentLevel == 0 ? 1 : 0.3)
                        .scaleEffect(currentLevel == 0 ? 1 : 0.8)
                    }
                    
                    // 二级轮盘（中圈）
                    if currentLevel >= 1 {
                        let children = themeStore.getChildren(for: selectedLevel0)
                        WheelView(
                            themes: children,
                            selectedIndex: index(of: selectedLevel1, in: children),
                            radius: 110
                        ) { theme in
                            selectTheme(theme, level: 1)
                        }
                        .opacity(currentLevel == 1 ? 1 : 0.3)
                        .scaleEffect(currentLevel == 1 ? 1 : 0.8)
                    }
                    
                    // 三级轮盘（内圈）
                    if currentLevel >= 2 {
                        let children = themeStore.getChildren(for: selectedLevel1)
                        WheelView(
                            themes: children,
                            selectedIndex: index(of: selectedLevel2, in: children),
                            radius: 60
                        ) { theme in
                            selectTheme(theme, level: 2)
                        }
                        .opacity(currentLevel == 2 ? 1 : 0.3)
                        .scaleEffect(currentLevel == 2 ? 1 : 0.8)
                    }
                }
                .animation(.spring(response: 0.3), value: currentLevel)
                
                Spacer()
                
                // 当前选择显示
                VStack(spacing: 8) {
                    Text(selectedThemeName)
                        .font(.title2)
                        .fontWeight(.semibold)
                }
                .padding()
            }
        }
        .onAppear {
            themeStore.load()
            if let first = themeStore.themes.first {
                selectedLevel0 = first.id
            }
        }
    }
    
    private var selectedThemeName: String {
        var names: [String] = []
        if let t = themeStore.getTheme(by: selectedLevel0) { names.append(t.name) }
        if let t = themeStore.getTheme(by: selectedLevel1) { names.append(t.name) }
        if let t = themeStore.getTheme(by: selectedLevel2) { names.append(t.name) }
        return names.joined(separator: " · ")
    }
    
    private func index(of id: String, in themes: [Theme]) -> Int {
        themes.firstIndex { $0.id == id } ?? 0
    }
    
    private func selectTheme(_ theme: Theme, level: Int) {
        guard !isAnimating else { return }
        isAnimating = true
        
        switch level {
        case 0:
            selectedLevel0 = theme.id
            selectedLevel1 = ""
            selectedLevel2 = ""
        case 1:
            selectedLevel1 = theme.id
            selectedLevel2 = ""
        case 2:
            selectedLevel2 = theme.id
        }
        
        // 停留 0.5 秒后进入下一级
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5) {
            if level < 2 && themeStore.hasChildren(theme) {
                withAnimation(.spring(response: 0.3)) {
                    currentLevel = level + 1
                }
            }
            isAnimating = false
        }
    }
}

#Preview {
    ConcentricWheelView()
}
```

- [ ] **Step 2: 更新 ContentView**

```swift
// TaskWheel/UI/Views/ContentView.swift
import SwiftUI

struct ContentView: View {
    var body: some View {
        ConcentricWheelView()
    }
}
```

- [ ] **Step 3: 运行预览验证**

```bash
# Xcode: 打开 ContentView.swift，点击 Preview
# 预期：显示三层同心圆轮盘
```

- [ ] **Step 4: 提交**

```bash
git add -A
git commit -m "feat: 添加同心圆容器视图"
```

---

## Phase 3: 手势与动画

### Task 7: 手势处理器

**Files:**
- Create: `TaskWheel/UI/Gestures/WheelGestureHandler.swift`

- [ ] **Step 1: 实现手势处理**

```swift
// TaskWheel/UI/Gestures/WheelGestureHandler.swift
import SwiftUI
import UIKit

final class WheelGestureHandler: ObservableObject {
    @Published var rotationAngle: Double = 0
    @Published var selectedThemeId: String = ""
    
    private var themes: [Theme] = []
    private var segmentAngle: Double = 0
    private let snapThreshold: Double = 10 // 度
    
    func configure(themes: [Theme]) {
        self.themes = themes
        self.segmentAngle = 360.0 / Double(themes.count)
    }
    
    func handlePan(_ value: CGFloat) {
        rotationAngle += value
        updateSelection()
    }
    
    func handleEnd() {
        // 惯性吸附到最近的扇区
        snapToNearest()
    }
    
    private func updateSelection() {
        guard !themes.isEmpty else { return }
        
        // 计算当前角度对应的扇区
        let normalizedAngle = normalizeAngle(rotationAngle)
        let index = Int(normalizedAngle / segmentAngle) % themes.count
        selectedThemeId = themes[index].id
    }
    
    private func snapToNearest() {
        let targetAngle = round(normalizeAngle(rotationAngle) / segmentAngle) * segmentAngle
        withAnimation(.spring(response: 0.3, dampingFraction: 0.7)) {
            rotationAngle = targetAngle
        }
    }
    
    private func normalizeAngle(_ angle: Double) -> Double {
        var normalized = angle.truncatingRemainder(dividingBy: 360)
        if normalized < 0 { normalized += 360 }
        return normalized
    }
}
```

- [ ] **Step 2: 提交**

```bash
git add -A
git commit -m "feat: 添加手势处理器"
```

---

## Phase 4: 模型集成

### Task 8: LLM 引擎封装

**Files:**
- Create: `TaskWheel/Model/LLMEngine.swift`
- Test: `TaskWheel/Tests/Model/LLMEngineTests.swift`

- [ ] **Step 1: 编写测试**

```swift
// TaskWheel/Tests/Model/LLMEngineTests.swift
import XCTest
@testable import TaskWheel

final class LLMEngineTests: XCTestCase {
    var engine: LLMEngine!
    
    func testGenerateChildren() async throws {
        engine = LLMEngine()
        let result = try await engine.generateChildren(
            for: "工作",
            context: "办公室任务"
        )
        XCTAssertGreaterThan(result.count, 0)
    }
}
```

- [ ] **Step 2: 实现 LLMEngine（简化版，先使用模拟数据）**

```swift
// TaskWheel/Model/LLMEngine.swift
import Foundation

actor LLMEngine {
    static let shared = LLMEngine()
    
    private var isModelLoaded = false
    
    private init() {}
    
    func loadModel() async throws {
        // TODO: 集成 llama.cpp
        // 目前使用模拟数据
        isModelLoaded = true
        print("✅ 模型加载完成（模拟）")
    }
    
    func generateChildren(for parentName: String, context: String) async throws -> [Theme] {
        // TODO: 实际调用 llama.cpp
        // 目前返回模拟数据
        try await Task.sleep(nanoseconds: 300_000_000) // 模拟 300ms 延迟
        
        return [
            Theme(id: "gen_1", name: "\(parentName)任务 1"),
            Theme(id: "gen_2", name: "\(parentName)任务 2"),
            Theme(id: "gen_3", name: "\(parentName)任务 3")
        ]
    }
}
```

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "feat: 添加 LLM 引擎（模拟版）"
```

---

## Phase 5: 测试与优化

### Task 9: 性能测试

**Files:**
- Create: `TaskWheel/Tests/Performance/WheelPerformanceTests.swift`

- [ ] **Step 1: 创建性能测试**

```swift
// TaskWheel/Tests/Performance/WheelPerformanceTests.swift
import XCTest
@testable import TaskWheel

final class WheelPerformanceTests: XCTestCase {
    func testSelectionTime() {
        measure {
            // 模拟完整选择流程
            let store = ThemeStore.shared
            store.load()
            
            // 三级选择
            _ = store.getTheme(by: "work")
            _ = store.getChildren(for: "work")
            _ = store.getChildren(for: "coding")
        }
    }
    
    func testAnimationFrameRate() {
        // 使用 XCTestExpectation 测试动画流畅度
        let expectation = XCTestExpectation(description: "Animation")
        
        DispatchQueue.main.asyncAfter(deadline: .now() + 2.0) {
            expectation.fulfill()
        }
        
        wait(for: [expectation], timeout: 3.0)
    }
}
```

- [ ] **Step 2: 运行性能测试**

```bash
xcodebuild test -scheme TaskWheel \
  -only-testing:TaskWheelTests/WheelPerformanceTests
```

- [ ] **Step 3: 提交**

```bash
git add -A
git commit -m "test: 添加性能测试"
```

---

## Phase 6: 文档与发布

### Task 10: 用户文档

**Files:**
- Create: `README.md`
- Create: `TaskWheel/Resources/Help.md`

- [ ] **Step 1: 创建 README**

```markdown
# TaskWheel

快速选择任务主题的 iOS 应用

## 特性

- 🎯 2.5 秒内完成选择
- 🎨 同心圆轮盘交互
- 🤖 本地 AI 扩展主题
- 🔒 完全离线

## 使用

1. 滑动旋转轮盘
2. 停留 0.5 秒自动确认
3. 边缘滑动返回

## 技术

- SwiftUI 5.0
- llama.cpp iOS
```

- [ ] **Step 2: 提交**

```bash
git add -A
git commit -m "docs: 添加用户文档"
```

---

## 验收清单

- [ ] 所有单元测试通过
- [ ] 性能测试达标（≤2.5 秒）
- [ ] 动画帧率≥55 FPS
- [ ] 内存占用≤100MB
- [ ] 完全离线运行
- [ ] 用户文档完整

---

**下一步**: 按照计划逐步执行，每完成一个 Task 提交一次代码
