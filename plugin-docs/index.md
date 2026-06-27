---
layout: home

hero:
  name: 枢 插件开发
  text: 为枢构建强大的插件
  tagline: 基于沙箱隔离与显式授权的安全插件体系
  actions:
    - theme: brand
      text: 快速上手
      link: /quickstart
    - theme: alt
      text: 查看 API 参考
      link: /host-api

features:
  - title: 沙箱安全隔离
    details: 插件运行在 sandbox="allow-scripts" 的 iframe 里，无法访问 Node、文件系统或 Tauri，确保宿主系统安全。
  - title: 显式权限授权
    details: 安装时用户逐项授权插件所需能力，权限取 granted ∩ declared 交集，技术层面完全隔离。
  - title: 灵活的插件类型
    details: 支持 UI 插件（交互界面）和逻辑插件（输入查询→结果列表）两种类型，覆盖大多数场景。
---
