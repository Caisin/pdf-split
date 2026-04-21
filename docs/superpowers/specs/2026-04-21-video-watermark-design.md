# 批量视频水印设计

## 目标
为当前桌面工具新增一个独立的“批量视频水印”顶级 Tab，使用 `Imgs::overlay_slanted_watermark_onto_videos_with_progress(...)` 实现目录级批量视频打水印，并保持 UI 在处理期间不被阻塞。

## 非目标
- 不把视频水印合并进现有“批量图片水印”页面
- 不引入视频裁剪、转码参数、码率控制等额外视频编辑能力
- 不新增独立的视频播放器预览器
- 不修改 `kx-image` 上游接口签名

## 用户界面设计

### 1. 顶级入口
新增一个新的顶级 Tab：
- 名称：`批量视频水印`

它和现有：
- 按页导出
- 提取内嵌图片
- 文字水印
- 批量图片水印

并列出现，不做二级切换。

### 2. 页面布局
页面整体风格与“批量图片水印”保持一致，采用紧凑卡片和网格布局：
- 输入目录
- 输出目录
- 水印文字
- 水印行数
- 是否铺满画面
- 透明度
- 条间距（字符倍数）
- 行间距（行高倍数）
- 预览区域
- 进度条
- 状态文案
- 开始按钮

布局原则：
- 输入/输出目录使用 `picker-grid`
- 短字段进入紧凑 `field-grid`
- 文本域保留整行
- 进度与状态展示模式对齐现有批量图片水印体验

## 预览设计

### 1. 预览来源
预览区域直接使用输入目录中**第一个可处理视频文件的首帧**作为预览底图。

### 2. 预览渲染方式
预览不单独维护一套视频水印渲染逻辑，而是：
1. 读取第一个视频首帧
2. 将首帧当作图片输入
3. 复用视频水印底层对应的图片 slanted watermark 渲染路径生成预览结果

目标是保证：
- 预览参数和实际视频批处理参数一致
- 预览视觉尽量接近最终输出效果
- 不把完整 ffmpeg 视频处理流程放进参数联动预览里

### 3. 预览行为
- 选择输入目录后自动尝试加载第一个视频首帧
- 参数变化后防抖刷新预览
- 若目录内无支持视频文件，则提示无法预览
- 若首帧提取失败，则展示失败提示，但不影响后续批处理提交

## 后端设计

### 1. Tauri 命令
新增独立命令，例如：
- `add_slanted_watermark_to_videos`

命令职责：
- 校验输入目录/输出目录/参数
- 使用 `spawn_blocking` 执行长耗时批处理
- 调用 `Imgs::overlay_slanted_watermark_onto_videos_with_progress(...)`
- 将进度事件持续发回前端
- 返回最终批处理结果汇总

### 2. 进度事件
新增独立事件名：
- `batch-video-watermark-progress`

事件负载至少包含：
- `scannedFileCount`
- `processedFileCount`
- `successCount`
- `generatedOverlayCount`
- `reusedOverlayCount`
- `currentFile`

这样可以与图片/PDF 批处理事件完全隔离，避免前端监听混淆。

### 3. 异步约束
必须与批量图片水印一致：
- 主线程不做同步长循环
- UI 提交后立即进入“处理中”状态
- 处理过程通过事件驱动更新进度
- 避免目录较大时卡住界面

## 数据模型

### 1. 输入模型
新增前后端输入类型：
- `BatchVideoWatermarkInput`

字段直接对齐 `SlantedWatermarkOptions` 所需参数，而不是再引入旧的字号比例/旋转比例适配层：
- `inputDir`
- `outputDir`
- `watermarkText`
- `watermarkLineCount`
- `watermarkFullScreen`
- `watermarkOpacity`
- `watermarkStripeGapChars`
- `watermarkRowGapLines`

### 2. 结果模型
新增：
- `BatchVideoWatermarkResult`

字段建议与上游结果一致：
- `scannedFileCount`
- `successCount`
- `generatedOverlayCount`
- `reusedOverlayCount`
- `outputDir`

### 3. 进度模型
新增：
- `BatchVideoWatermarkProgress`

字段与事件负载一致，用于前端进度显示。

## 实现边界

### 前端
需要修改或新增：
- `toolTabs`：新增 tab 定义
- `App.tsx`：挂载新工具页面
- 新建 `BatchVideoWatermarkTool.tsx`
- `tool-types.ts`：新增视频批处理类型
- 可能补充少量 `App.css` 以复用现有预览/进度样式

### 后端
需要修改或新增：
- `models.rs`：新增视频批处理输入/结果/进度模型
- `commands.rs`：新增视频批处理命令、首帧预览命令/辅助逻辑、进度事件发射
- `lib.rs`：注册新命令

## 测试设计

### 前端测试
覆盖：
1. 新 tab 渲染成功
2. 默认禁用提交
3. 输入目录与输出目录相同则禁用
4. 选择输入目录后加载第一个视频首帧预览
5. 参数变化触发预览刷新
6. 提交时调用正确命令
7. 处理中按钮禁用并显示进度条
8. 进度事件能更新状态文案

### Rust 测试
覆盖：
1. 参数校验
2. 输入/输出目录冲突校验
3. 无视频文件时报错
4. 进度回调透传
5. 命令走异步批处理包装而不是同步阻塞路径
6. 预览辅助逻辑在首帧读取失败或无视频时给出明确错误

## 风险与约束
- 视频首帧提取依赖底层 ffmpeg/ffprobe 能力，预览失败时必须优雅降级
- 批量视频处理耗时明显高于图片，必须坚持事件驱动进度更新
- 预览只代表首帧效果，不承诺覆盖视频内所有场景变化
- 上游 `kx-image` 已经实现 overlay cache 复用，当前接入层应尽量薄封装，不重复维护缓存逻辑

## 验收标准
1. 新增“批量视频水印”顶级 Tab
2. 可选择输入/输出目录，递归批量处理视频
3. 参数模型直接对应 `SlantedWatermarkOptions`
4. 预览区能显示第一个视频首帧叠加水印后的效果
5. 批处理过程不会卡死界面
6. 进度条和状态文案会随事件实时更新
7. 前端测试、前端构建、Rust 测试通过
