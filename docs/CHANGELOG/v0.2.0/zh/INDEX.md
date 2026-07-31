# Ensign v0.2.0

Ensign 的冷启动闭合。服务端镜像持有两个彼此独立的进程命令：`bootstrap` 组合
Keel 的热点与调用方的 artifact 目的地，`serve` 只验证前置条件，缺一即拒绝打开
监听。sudo 与 OIDC signing 共享一套编排语法，但从不共享 custody、挂载、访问、
生命周期或域归属。

Helm 交付现在能在真集群上把这套仪式立起来。bootstrap 改为由 API Pod 自己持有的
initContainer，而不再是 hook 驱动的 Job —— `helm install --wait` 会先等资源
ready 再跑 post-install hook，那个 Job 因此从未被创建。sudo、signing、store 三个
Secret 与 store 卷都带上 `helm.sh/resource-policy: keep`，于是 uninstall 之后
custody 仍在，重装认领原有 estate 而不是重新 mint。store 凭据一次生成、两处复用，
chart 也不再部署 web 平面。

身份本身被拆开。一个人**是谁**、如何**登录**、如何**被看见**从一行拆成三件事，
access token 也开始携带它真正被请求的那个 resource。Ensign 同时把配置面从 Keel
收回自持，并迁到 Sealkit 0.3。

这是第一个经共享 release lane 出门的版本。仓库在 `plumb.toml` 里声明产品、
authority、二进制、目标平台与 skill，两个薄壳把时序交给共享 Actions workflow。
`ensign` 客户端与这份 brief 从此是 release 的可安装产物，不再是靠人复制的文件。
