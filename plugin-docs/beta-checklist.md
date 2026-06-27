# Plugin SDK Beta 发布清单

目标：确认 pc-tool 可以作为一个可试用的插件 SDK 发布给早期插件作者。

## 必须通过

- [ ] `pnpm test`
- [ ] `pnpm check`
- [ ] `pnpm build`
- [ ] `pnpm docs:build`
- [ ] `cd src-tauri && cargo test`
- [ ] `pnpm tauri:test` 可打开 `/test` 测试窗口
- [ ] `/test` 可对 `/tmp/pc-tool-json-preview.pcp` 完成 Inspect / Install / Uninstall
- [ ] 正常启动 `pnpm tauri dev` 后，`Cmd+Shift+Space` 可唤起窗口
- [ ] 输入 `json` 可打开 JSON 编辑器
- [ ] 输入 `data` 可打开“存储与文件示例”，保存后重新打开仍能读回内容

## 发布前人工冒烟

1. 打包一个插件：

```bash
cd plugins/json-preview
zip -qr /tmp/pc-tool-json-preview.pcp .
```

2. 运行测试窗口：

```bash
cd ../..
pnpm tauri:test
```

3. 在测试窗口依次点击 Inspect、Install、Uninstall。

4. 运行正常启动器：

```bash
pnpm tauri dev
```

5. 用 `Cmd+Shift+Space` 唤起，分别测试 `json`、`hosts`、`data` 三个内置插件。

## Beta 不做

- 插件市场
- 自动更新
- 发布者身份和签名
- 云同步
- Windows / Linux 适配

这些等 SDK 的安装、授权、能力桥和文档被早期作者验证后再做。
