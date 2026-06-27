# Shu Plugin Template

最小插件仓库模板。

## 开发

1. 修改 `plugin.json`：
   - `id` 使用反向域名格式，例如 `com.you.my-plugin`
   - `version` 跟随 release tag
   - `permissions` 只填实际需要的能力
2. 修改 `index.html`。
3. 推送 tag：

```bash
git tag v0.1.0
git push origin v0.1.0
```

GitHub Actions 会把仓库根目录打包成 `.pcp` 并发布到 GitHub Release。

## 提交到官方 registry

Release 成功后，到官方 `shu-registry` 仓库提交你的插件仓库地址即可。
