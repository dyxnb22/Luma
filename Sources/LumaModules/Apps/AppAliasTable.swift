import Foundation

public enum AppAliasTable {
    private static let aliasByBundleID: [String: [String]] = [
        "com.tencent.xinWeChat": ["微信", "wechat", "weixin", "wx"],
        "com.tencent.qq": ["QQ", "腾讯QQ", "qq"],
        "com.alibaba.DingTalkMac": ["钉钉", "dingtalk", "dd"],
        "com.bytedance.lark": ["飞书", "lark", "feishu", "fs"],
        "com.netease.163music": ["网易云音乐", "网易云", "neteasemusic", "wyy"],
        "com.tencent.meeting.officialmac": ["腾讯会议", "tencentmeeting", "txhy"],
        "com.tencent.WeWorkMac": ["企业微信", "wecom", "qywx"],
        "com.kingsoft.wpsoffice.mac": ["WPS", "wps office", "wps"],
        "com.microsoft.VSCode": ["VS Code", "vscode", "vsc"],
        "com.jetbrains.intellij": ["IntelliJ IDEA", "idea"],
        "com.jetbrains.intellij.ce": ["IntelliJ IDEA", "idea"],
        "com.google.Chrome": ["Chrome", "谷歌浏览器", "gg"],
        "org.mozilla.firefox": ["Firefox", "火狐"],
        "com.apple.Safari": ["Safari", "苹果浏览器"],
        "com.adobe.Photoshop": ["Photoshop", "ps", "psd"],
        "com.adobe.PhotoshopLightroom": ["Lightroom", "lr"],
        "com.apple.finder": ["Finder", "finder"],
        "com.apple.dt.Xcode": ["Xcode", "xc"],
        "com.apple.iWork.Keynote": ["Keynote", "讲演"],
        "com.apple.iWork.Numbers": ["Numbers", "表格"],
        "com.apple.iWork.Pages": ["Pages", "文稿"],
        "com.spotify.client": ["Spotify", "声田"],
        "com.bohemiancoding.sketch3": ["Sketch"],
        "com.figma.Desktop": ["Figma"],
        "com.slack.Slack": ["Slack"],
        "com.docker.docker": ["Docker"],
        "com.postmanlabs.mac": ["Postman"],
        "com.tencent.tenvideo": ["腾讯视频"],
        "com.youku.mac": ["优酷"],
        "com.baidu.BaiduNetdisk-mac": ["百度网盘", "baidunetdisk"],
    ]

    public static func aliases(forBundleID bundleID: String, name: String, zhNames: [String]) -> [String] {
        aliasByBundleID[bundleID] ?? []
    }
}
