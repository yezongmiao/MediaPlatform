use serde::{de::DeserializeOwned, Serialize};

pub fn parse<T: DeserializeOwned>(raw: &str) -> Result<(T, String), String> {
    let normalized = raw.replace("\r\n", "\n");
    let rest = normalized
        .strip_prefix("---\n")
        .ok_or_else(|| "文件缺少 YAML frontmatter 起始分隔线".to_string())?;

    let mut offset = 0usize;
    let mut yaml = String::new();
    for line in rest.split_inclusive('\n') {
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed == "---" {
            let body = rest[offset + line.len()..]
                .strip_prefix('\n')
                .unwrap_or(&rest[offset + line.len()..])
                .to_string();
            let meta = serde_yaml::from_str::<T>(&yaml)
                .map_err(|e| format!("frontmatter 解析失败：{e}"))?;
            return Ok((meta, body));
        }
        yaml.push_str(line);
        offset += line.len();
    }

    // Allow a frontmatter block at EOF without a trailing newline.
    if yaml.trim_end().ends_with("---") {
        let trimmed = yaml.trim_end();
        let yaml = trimmed[..trimmed.len() - 3].to_string();
        let meta =
            serde_yaml::from_str::<T>(&yaml).map_err(|e| format!("frontmatter 解析失败：{e}"))?;
        return Ok((meta, String::new()));
    }

    Err("文件缺少 YAML frontmatter 结束分隔线".to_string())
}

pub fn render<T: Serialize>(meta: &T, body: &str) -> Result<String, String> {
    let mut yaml =
        serde_yaml::to_string(meta).map_err(|e| format!("frontmatter 序列化失败：{e}"))?;
    if let Some(stripped) = yaml.strip_prefix("---\n") {
        yaml = stripped.to_string();
    }
    let body = body.trim_end_matches(['\r', '\n']);
    Ok(format!("---\n{}\n---\n\n{}\n", yaml.trim_end(), body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    #[serde(rename_all = "camelCase")]
    struct Meta {
        id: String,
        title: String,
        #[serde(default)]
        tags: Vec<String>,
    }

    #[test]
    fn roundtrip_with_chinese_and_dashes() {
        let meta = Meta {
            id: "01ABC".into(),
            title: "想法：中文 & 空格".into(),
            tags: vec!["灵感".into()],
        };
        let raw = render(&meta, "第一行\n\n---\n正文里的分隔线不应截断\n---\n最后").unwrap();
        let (parsed, body) = parse::<Meta>(&raw).unwrap();
        assert_eq!(parsed, meta);
        assert_eq!(
            body.trim(),
            "第一行\n\n---\n正文里的分隔线不应截断\n---\n最后"
        );
    }

    #[test]
    fn rejects_missing_delimiters() {
        assert!(parse::<Meta>("hello").is_err());
        assert!(parse::<Meta>("---\nid: x\n").is_err());
    }

    #[test]
    fn handles_crlf() {
        let raw = "---\r\nid: x\r\ntitle: 标题\r\n---\r\n\r\n正文";
        let (meta, body) = parse::<Meta>(raw).unwrap();
        assert_eq!(meta.title, "标题");
        assert_eq!(body, "正文");
    }
}
