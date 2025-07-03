use std::default;

use crate::taskmanager::{PushRequest, PushType, TaskManager, TaskRequest, TaskStatus};
use reqwest::Client;
use serde::{Deserialize, Serialize};

//推送token
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PushConfig {
    pub enabled: bool,
    pub bark_token: String,
    pub pushplus_token: String,
    pub fangtang_token: String,
    pub dingtalk_token: String,
    pub wechat_token: String,
    pub gotify_config: GotifyConfig,
    pub smtp_config: SmtpConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GotifyConfig {
    pub gotify_url: String,
    pub gotify_token: String,
}

impl GotifyConfig {
    pub fn new() -> Self {
        Self {
            gotify_url: String::new(),
            gotify_token: String::new(),
        }
    }
}
//邮箱配置(属于pushconfig)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub smtp_server: String,
    pub smtp_port: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_from: String,
    pub smtp_to: String,
}

impl PushConfig {
    pub fn new() -> Self {
        Self {
            enabled: false,
            bark_token: String::new(),
            pushplus_token: String::new(),
            fangtang_token: String::new(),
            dingtalk_token: String::new(),
            wechat_token: String::new(),
            gotify_config: GotifyConfig::new(),
            smtp_config: SmtpConfig::new(),
        }
    }

    pub fn push_all(
        &self,
        title: &str,
        message: &str,
        jump_url: &Option<String>,
        task_manager: &mut dyn TaskManager,
    ) {
        if !self.enabled {
            return;
        }
        let push_request = TaskRequest::PushRequest(PushRequest {
            title: title.to_string(),
            message: message.to_string(),
            jump_url: jump_url.clone(),
            push_config: self.clone(),
            push_type: PushType::All,
            status: TaskStatus::Pending,
        });
        match task_manager.submit_task(push_request) {
            Ok(task_id) => {
                log::debug!("提交全渠道推送任务成功，任务ID: {}", task_id);
            }
            Err(e) => {
                log::error!("提交推送任务失败: {}", e);
            }
        }
    }

    pub async fn push_all_async(
        &self,
        title: &str,
        message: &str,
        jump_url: &Option<String>,
    ) -> (bool, String) {
        let mut success_count = 0;
        let mut failure_count = 0;
        let mut failures = Vec::new();

        if !self.bark_token.is_empty() {
            let (success, msg) = self.push_bark(title, message).await;
            if success {
                success_count += 1;
            } else {
                failure_count += 1;
                failures.push(format!("Bark推送出错: {}", msg));
            }
        }

        if !self.pushplus_token.is_empty() {
            let (success, msg) = self.push_pushplus(title, message).await;
            if success {
                success_count += 1;
            } else {
                failure_count += 1;
                failures.push(format!("PushPlus推送出错: {}", msg));
            }
        }
        if !self.fangtang_token.is_empty() {
            let (success, msg) = self.push_fangtang(title, message).await;
            if success {
                success_count += 1;
            } else {
                failure_count += 1;
                failures.push(format!("Fangtang推送出错: {}", msg));
            }
        }
        if !self.dingtalk_token.is_empty() {
            let (success, msg) = self.push_dingtalk(title, message).await;
            if success {
                success_count += 1;
            } else {
                failure_count += 1;
                failures.push(format!("Dingtalk推送出错: {}", msg));
            }
        }
        if !self.wechat_token.is_empty() {
            let (success, msg) = self.push_wechat(title, message).await;
            if success {
                success_count += 1;
            } else {
                failure_count += 1;
                failures.push(format!("WeChat推送出错: {}", msg));
            }
        }
        if !self.smtp_config.smtp_server.is_empty() {
            let (success, msg) = self.push_smtp(title, message).await;
            if success {
                success_count += 1;
            } else {
                failure_count += 1;
                failures.push(format!("SMTP推送出错: {}", msg));
            }
        }
        if !self.gotify_config.gotify_token.is_empty() {
            let (success, msg) = self.push_gotify(title, message, jump_url).await;
            if success {
                success_count += 1;
            } else {
                failure_count += 1;
                failures.push(format!("Gotify推送出错: {}", msg));
            }
        }
        if success_count == 0 {
            return (
                false,
                format!(
                    "{} 成功 / {} 失败。失败详情: {}",
                    success_count,
                    failure_count,
                    failures.join("; ")
                ),
            );
        } else {
            return (true, format!("全部 {} 个渠道推送成功", success_count));
        }
    }
    pub async fn push_gotify(
        &self,
        title: &str,
        message: &str,
        jump_url: &Option<String>,
    ) -> (bool, String) {
        let mut default_headers = reqwest::header::HeaderMap::new();
        let jump_url_real = match jump_url {
            Some(url) => url,
            None => &"bilibili://mall/web?url=https://www.bilibili.com".to_string(),
        };
        let push_target_url = if self.gotify_config.clone().gotify_url.contains("http") {
            self.gotify_config.clone().gotify_url
        } else {
            format!("http://{}", self.gotify_config.clone().gotify_url)
        };
        default_headers.insert(
            "Content-Type",
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        default_headers.insert(
            "Authorization",
            reqwest::header::HeaderValue::from_str(&format!(
                "Bearer {}",
                self.gotify_config.gotify_token
            ))
            .unwrap(),
        );
        let client_builder = Client::builder()
            .default_headers(default_headers)
            .timeout(std::time::Duration::from_secs(20));
        let data = serde_json::json!({
            "message": message,
            "title": title,
            "priority": 9,
            "extras": {
            "client::notification": {
                "click": {"url": jump_url_real},
            },
            "android::action": {
                "onReceive": {"intentUrl": jump_url_real}
            }
        }
        });
        let client = match client_builder.build() {
            Ok(client) => client,
            Err(e) => return (false, format!("创建HTTP客户端失败: {}", e)),
        };
        let url = format!("{}/message", push_target_url);

        match client.post(&url).json(&data).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Ok(text) => {
                        log::debug!("Gotify 推送响应: 状态码 {}, 内容: {}", status, text);
                        if status.is_success() {
                            (true, "推送成功".to_string())
                        } else {
                            (false, format!("推送失败，状态码: {}", status))
                        }
                    }
                    Err(e) => (false, format!("读取响应失败: {}", e)),
                }
            }
            Err(e) => (false, format!("推送失败: {}", e)),
        }
    }
    pub async fn push_bark(&self, title: &str, message: &str) -> (bool, String) {
        let client = Client::new();
        let data = serde_json::json!({
            "title":title,
            "body":message,
            "level":"timeSensitive",
           /*   #推送中断级别。
                #active：默认值，系统会立即亮屏显示通知
                #timeSensitive：时效性通知，可在专注状态下显示通知。
                #passive：仅将通知添加到通知列表，不会亮屏提醒。  */
            "badge":1,
            "icon":"https://sr.mihoyo.com/favicon-mi.ico",
            "group":"biliticket",
            "isArchive":1,

        });
        let url = format!("https://api.day.app/{}/", self.bark_token);
        match client.post(&url).json(&data).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Ok(text) => {
                        log::debug!("Bark 推送响应: 状态码 {}, 内容: {}", status, text);
                        if status.is_success() {
                            (true, "推送成功".to_string())
                        } else {
                            (false, format!("推送失败，状态码: {}", status))
                        }
                    }
                    Err(e) => (false, format!("读取响应失败: {}", e)),
                }
            }
            Err(e) => (false, format!("推送失败: {}", e)),
        }
    }

    pub async fn push_pushplus(&self, title: &str, message: &str) -> (bool, String) {
        let client = Client::new();
        let url = "http://www.pushplus.plus/send";
        let data = serde_json::json!({
            "token":self.pushplus_token,
            "title":title,
            "content":message,
        });
        match client.post(url).json(&data).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Ok(text) => {
                        log::debug!("PushPlus 推送响应: 状态码 {}, 内容: {}", status, text);
                        if status.is_success() {
                            (true, "推送成功".to_string())
                        } else {
                            (false, format!("推送失败，状态码: {}", status))
                        }
                    }
                    Err(e) => (false, format!("读取响应失败: {}", e)),
                }
            }
            Err(e) => (false, format!("推送失败: {}", e)),
        }
    }

    pub async fn push_fangtang(&self, title: &str, message: &str) -> (bool, String) {
        let client = Client::new();
        let url = format!("https://sctapi.ftqq.com/{}.send", self.fangtang_token);
        let data = serde_json::json!({
            "title":title,
            "desp":message,
            "noip":1
        });
        match client.post(url).json(&data).send().await {
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Ok(text) => {
                        log::debug!("Fangtang 推送响应: 状态码 {}, 内容: {}", status, text);
                        if status.is_success() {
                            (true, "推送成功".to_string())
                        } else {
                            (false, format!("推送失败，状态码: {}", status))
                        }
                    }
                    Err(e) => (false, format!("读取响应失败: {}", e)),
                }
            }
            Err(e) => (false, format!("推送失败: {}", e)),
        }
    }

    pub async fn push_dingtalk(&self, title: &str, message: &str) -> (bool, String) {
        let client = Client::new();
        let url = format!(
            "https://oapi.dingtalk.com/robot/send?access_token={}",
            self.dingtalk_token
        );
        let data = serde_json::json!({
            "msgtype":"text",
            "text":{
                "content":format!("{} \n {}", title, message)
            }
        });
        match client
            .post(url)
            .json(&data)
            .header("Content-Type", "application/json")
            .header("Charset", "UTF-8")
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Ok(text) => {
                        log::debug!("钉钉推送响应: 状态码 {}, 内容: {}", status, text);
                        if status.is_success() {
                            (true, "推送成功".to_string())
                        } else {
                            (false, format!("推送失败，状态码: {}", status))
                        }
                    }
                    Err(e) => (false, format!("读取响应失败: {}", e)),
                }
            }
            Err(e) => (false, format!("推送失败: {}", e)),
        }
    }

    pub async fn push_wechat(&self, title: &str, message: &str) -> (bool, String) {
        let client = Client::new();
        let url = format!(
            "https://qyapi.weixin.qq.com/cgi-bin/webhook/send?key={}",
            self.wechat_token
        );
        let data = serde_json::json!({
            "msgtype":"text",
            "text":{
                "content":format!("{} \n {}", title, message)
            }
        });
        match client
            .post(url)
            .json(&data)
            .header("Content-Type", "application/json")
            .header("Charset", "UTF-8")
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                match resp.text().await {
                    Ok(text) => {
                        log::debug!("微信推送响应: 状态码 {}, 内容: {}", status, text);
                        if status.is_success() {
                            (true, "推送成功".to_string())
                        } else {
                            (false, format!("推送失败，状态码: {}", status))
                        }
                    }
                    Err(e) => (false, format!("读取响应失败: {}", e)),
                }
            }
            Err(e) => (false, format!("推送失败: {}", e)),
        }
    }

    pub async fn push_smtp(&self, title: &str, message: &str) -> (bool, String) {
        return (false, "SMTP推送功能未实现".to_string());
    }
}

impl SmtpConfig {
    pub fn new() -> Self {
        Self {
            smtp_server: String::new(),
            smtp_port: String::new(),
            smtp_username: String::new(),
            smtp_password: String::new(),
            smtp_from: String::new(),
            smtp_to: String::new(),
        }
    }
}
