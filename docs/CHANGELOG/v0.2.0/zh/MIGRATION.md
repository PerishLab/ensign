# 迁移到 Ensign v0.2.0

现存部署无需迁移：v0.2.0 是第一个经 release lane 分发的版本，此前没有任何版本
是从 lane 里装出来的。

若要升级一个在此之前安装的 chart，有三件事需要知道。

API 容器镜像的 entrypoint 变了。原为 `["ensign-api", "/ensign"]`，把 root 钉成
第一个位置参数，与新的子命令冲突；现为 `ENTRYPOINT ["ensign-api"]` 加
`CMD ["serve", "/ensign"]`。曾覆盖 command 的部署必须显式传 `bootstrap` 或
`serve`。

bootstrap 从 Helm hook Job 移入 API Pod 的 initContainer。用于分离两个工作负载的
`-api` ServiceAccount / Role / RoleBinding 一并删除 —— 一个 Pod 只有一个
ServiceAccount。它们原本强制的那条边界（提供服务的进程永不持有 sudo）现在由二进制
强制：运行时按名拒绝 `sudo` artifact。

保留类 artifact 加上了 `helm.sh/resource-policy: keep`。sudo、signing、store 三个
Secret 与 store 卷现在能挺过 `helm uninstall`。删除它们是一次刻意的动作；在存活的
custody 之上重装会认领原有 estate，而不是封一个新的。

在同一个 tag 上迭代镜像时请设 `api.pullPolicy: Always`。chart 此前未声明拉取策略，
节点会一直跑它已经缓存的那个同名 tag 的二进制。
