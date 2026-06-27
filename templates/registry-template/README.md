# Shu Registry Template

官方 registry 的最小仓库模板。

## 作者提交插件

在 `submissions/` 里新增一个 JSON 文件：

```json
{
  "repo": "https://github.com/you/your-shu-plugin"
}
```

插件仓库需要使用 GitHub Release，并在 latest release 里提供一个 `.pcp` asset。

## 维护者更新 registry

PR 会运行 `validate-submissions.yml`：

1. 读取 `submissions/*.json`
2. 查每个仓库的 latest release
3. 找 `.pcp` asset
4. 运行 `pnpm registry:intake <asset-url> registry.json`
5. 校验生成的 `registry.json`

如果 CI 通过，维护者把生成的 `registry.json` diff 合并即可。
