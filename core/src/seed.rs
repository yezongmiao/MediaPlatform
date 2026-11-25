use std::path::Path;

use chrono::Utc;
use serde_json::json;
use tokio::fs;

use crate::error::{ApiError, ApiResult};
use crate::frontmatter;
use crate::models::{PresetMeta, SkillMeta};
use crate::store::atomic_write;

pub const SEED_VERSION: u32 = 1;

struct SeedSkill {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    applies_to: &'static [&'static str],
    content: &'static str,
}

struct SeedPreset {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    kind: &'static str,
    skill_ids: &'static [&'static str],
    content: &'static str,
}

const SKILLS: &[SeedSkill] = &[
    SeedSkill {
        id: "skill-hook",
        name: "强钩子开头",
        description: "前三秒/前两行必须让人停下来，提供 6 种可复用钩子模型。",
        applies_to: &["article", "script", "card", "video"],
        content: r#"# 强钩子开头

开头只做一件事：制造「这和我有关」或「这不合常理」的张力。从下面挑选其一，不要全部堆叠。

1. **反常识**：先说一个与大众直觉相反、但你有证据的判断。
2. **具体损失**：用可感知的代价开场，例如「我因为这个错误，白做了三个月」。
3. **场景代入**：让读者在脑内看到自己，例如「凌晨一点，你还在改第三条脚本」。
4. **结果前置**：先给结果，再拆过程；结果必须具体到数字或变化。
5. **冲突对话**：用一句真实对话切入，避免「大家好」。
6. **悬念缺口**：给信息但故意留一个关键缺口，正文必须补上。

约束：
- 不用「在这个信息爆炸的时代」「众所周知」等空话。
- 不虚构数据、身份、案例。没有真实细节时用可验证的观察替代。
- 首段不超过 80 字（口播不超过 2 句）。"#,
    },
    SeedSkill {
        id: "skill-structure",
        name: "金字塔叙事结构",
        description: "先结论后论据，适合知识类长文与中长视频，保证信息密度。",
        applies_to: &["article", "script", "video"],
        content: r#"# 金字塔叙事结构

按「结论 → 论据 → 行动」组织，而不是按时间流水账。

- **一句话结论**：读者看完能复述的主张，必须可被证伪或执行。
- **三层论据**：每层一个核心点，配一个例子/数据/对比；例子优先来自真实经历或公开来源。
- **反方处理**：主动指出一个常见反对意见，并给出你的边界条件。
- **收束**：给读者一个今天就能做的动作，或一个值得思考的问题。

检查：
1. 每个小标题能否独立成为一个信息点？
2. 任意删除一段，是否影响结论？不影响就删。
3. 是否存在「正确的废话」？换成具体的人、数、场景。"#,
    },
    SeedSkill {
        id: "skill-story",
        name: "故事化表达",
        description: "把观点装进人物、欲望、阻力与转折里，避免干瘪说教。",
        applies_to: &["article", "script", "video"],
        content: r#"# 故事化表达

不要编造故事。从真实经历、观察或公开案例中提取最小故事单元：

- **人物**：谁，处于什么具体处境。
- **欲望**：他想要什么，为什么现在想要。
- **阻力**：什么让他得不到，代价是什么。
- **转折**：哪个瞬间改变了他的判断/行动。
- **余味**：故事结束后，留给读者的那个问题。

写法：
- 多用动作与对话，少用形容词。
- 一个段落只推进一个节拍。
- 不在故事中间插入大段道理；道理放在故事之后，一两句即可。
- 涉及真实人物时做匿名化，不暴露隐私。"#,
    },
    SeedSkill {
        id: "skill-title",
        name: "标题打磨",
        description: "一次产出多组标题，兼顾信息量、点击动机与平台合规。",
        applies_to: &["article", "script", "card", "video"],
        content: r#"# 标题打磨

一次给 6 个候选，分别覆盖不同动机，不要只换同义词。

- **信息差型**：读者不知道但需要知道的关键事实。
- **方法型**：明确对象 + 明确收益 + 可执行范围。
- **反常识型**：与直觉冲突的判断，正文必须兑现。
- **代价型**：不改变的后果，避免恐吓。
- **身份型**：点名具体人群，越具体越好。
- **数字型**：数字必须真实、口径清楚。

自检：
- 去掉夸张词后还剩信息吗？
- 标题里的承诺，正文是否兑现？
- 是否触碰平台禁止的绝对化用语、医疗/金融保证性表述？
- 长度：公众号 16–26 字；短视频 12–20 字；文卡 8–18 字。"#,
    },
    SeedSkill {
        id: "skill-visual",
        name: "视觉提示词构建",
        description: "把抽象概念转成可执行的画面提示词，包含镜头、光线、材质与负面词。",
        applies_to: &["illustration", "card", "video"],
        content: r#"# 视觉提示词构建

每组提示词按「主体 → 场景 → 构图 → 光线 → 色彩 → 风格 → 画质」排列，并给出负面词。

- **主体**：一个明确视觉中心，不要同时出现三个以上主体。
- **场景**：具体地点/时间/天气，避免「美丽的背景」。
- **构图**：景别（特写/中景/全景）、机位（平视/俯拍）、留白位置（给标题留出区域）。
- **光线**：自然光/霓虹/伦勃朗光，说明方向与强弱。
- **色彩**：三色以内，给出主色与点缀色。
- **风格**：摄影、编辑插画、丝网印刷、胶片等，选一种为主。
- **画质**：分辨率、细节、镜头参数（可选）。

每组包含：
1. 中文画面描述（给创作者看）。
2. English prompt（给图像模型）。
3. Negative prompt。
4. 推荐比例与安全区说明。

禁止在画面中生成具体品牌 Logo、真实人物肖像或可识别面孔。"#,
    },
    SeedSkill {
        id: "skill-compliance",
        name: "事实与合规自检",
        description: "发布前检查数据来源、绝对化表述、隐私与平台风险。",
        applies_to: &["article", "script", "card", "video"],
        content: r#"# 事实与合规自检

在输出末尾附一行 `自检：` 列出以下项目，不要省略。

1. **数据**：每个数字能否找到来源？找不到就改为定性表达。
2. **案例**：是否真实？是否获得授权？无法确认时匿名化并注明「基于观察」。
3. **效果承诺**：删除「保证」「一定能」「零风险」等绝对化承诺。
4. **专业领域**：医疗、法律、投资内容必须加「非专业建议」提示。
5. **隐私**：不出现真实姓名、联系方式、单位内部信息、可识别住址。
6. **平台规则**：不诱导点赞/转发，不使用「最」「第一」等无依据排名。
7. **AI 标识**：若平台要求，提醒创作者标注 AI 辅助生成。

输出时不要伪造来源链接。没有来源就明确写「待补充来源」。"#,
    },
    SeedSkill {
        id: "skill-platform-remix",
        name: "多平台改写",
        description: "同一素材改写为公众号、小红书、抖音口播三种语体，保留核心观点。",
        applies_to: &["article", "script", "card", "video"],
        content: r#"# 多平台改写

保留一个核心观点，调整语体、节奏与结构，而不是简单删减。

- **公众号**：完整论证，小标题清晰，开头 2 行给钩子，结尾给行动或问题。
- **小红书**：第一人称、短段落、具体清单；标题 18 字内；避免夸张营销词。
- **抖音口播**：3 秒钩子 + 15 秒展开 + 5 秒收束；句子短，避免书面连接词；标注停顿点。
- **文卡图**：一句主张 + 一句补充，最多 3 行；留白给视觉。

改写时：
- 不改变事实与数字。
- 不把长文观点压缩成无法验证的断言。
- 每个平台单独给标题，不要复用同一标题。"#,
    },
];

const PRESETS: &[SeedPreset] = &[
    SeedPreset {
        id: "preset-article-deep",
        name: "公众号深度长文",
        description: "观点鲜明、论据扎实的 1500–2500 字长文，含标题与结尾行动。",
        kind: "article",
        skill_ids: &[
            "skill-hook",
            "skill-structure",
            "skill-title",
            "skill-compliance",
        ],
        content: r#"# 创作目标
写一篇可直接发布的公众号深度长文，让读者获得一个新判断，并愿意收藏或转发。

# 结构
1. 标题候选 3 个（16–26 字，风格不同）。
2. 开头钩子：不超过 2 段，制造张力。
3. 3–4 个主体小节：每节一个信息点，配真实例子/公开数据/对比。
4. 反方与边界：承认适用条件，避免绝对化。
5. 结尾：一句可复述的结论 + 一个今天可执行的动作。

# 输出格式
Markdown。一级标题为文章标题，小节用二级标题。不要输出创作说明、不要写「作为 AI」。
结尾附「自检：」一行，列出待核实项。

# 质量约束
- 总字数 1500–2500 字，除非用户另有要求。
- 每 300 字至少出现一个具体细节（人、场景、数字、动作）。
- 不编造来源、不虚构采访、不写空泛鸡汤。"#,
    },
    SeedPreset {
        id: "preset-script-spoken",
        name: "口播短视频脚本",
        description: "30–90 秒口播脚本，含 3 秒钩子、节奏标记与镜头提示。",
        kind: "script",
        skill_ids: &["skill-hook", "skill-story", "skill-compliance"],
        content: r#"# 创作目标
写一份可以直接对着镜头念的短视频口播脚本，时长 30–90 秒。

# 结构
- **标题候选**：3 个，12–20 字。
- **3 秒钩子**：一句话，必须具体。
- **正文**：按「问题 → 转折 → 方法/观点 → 收束」推进。
- **镜头提示**：每个意群标注景别/动作/字幕重点，用括号。
- **结尾**：一句互动问题，不要诱导点赞。

# 输出格式
表格或分栏 Markdown：时间码 | 口播 | 镜头/字幕。
口播句子短，单句不超过 30 字；标出停顿 `//`。

# 质量约束
- 总口播字数 = 时长 × 3.5 字/秒左右。
- 全程第二人称或第一人称，不要混用。
- 不虚构经历；不确定的地方标注「待补真实案例」。"#,
    },
    SeedPreset {
        id: "preset-illustration-scene",
        name: "插图场景提示词",
        description: "为文章/视频生成 3 组风格统一、可直接投喂图像模型的插图提示词。",
        kind: "illustration",
        skill_ids: &["skill-visual"],
        content: r#"# 创作目标
围绕项目主题，产出 3 组可直接用于图像生成模型的插图提示词，风格统一、可成套使用。

# 每组必须包含
1. 中文画面说明（2–3 句）。
2. English prompt（逗号分隔，包含主体/场景/构图/光线/色彩/风格/画质）。
3. Negative prompt。
4. 推荐比例（如 3:4、16:9）。
5. 标题安全区建议（顶部/底部/左侧留白）。

# 风格
编辑插画 + 纸张纹理 + 克制的暖色调；避免赛博霓虹、过度 3D、真实人脸。
三组之间保持同一色彩体系与材质，但景别不同（特写/中景/全景）。

# 输出格式
Markdown，每组用二级标题。不要生成图片，只输出提示词。"#,
    },
    SeedPreset {
        id: "preset-card-quote",
        name: "文卡图文案",
        description: "为文卡图生成 5 组短文案与视觉建议，适合 3:4 与 1:1 版式。",
        kind: "card",
        skill_ids: &["skill-hook", "skill-title"],
        content: r#"# 创作目标
为一张文卡图创作文案，让读者在 2 秒内抓住一句值得保存的话。

# 输出
给出 5 组方案，每组包含：
- **主张**：一句 8–18 字的核心句。
- **补充**：一句 12–30 字的解释或出处（没有出处就不编）。
- **视觉建议**：背景、字体气质、配色（三色以内）、留白位置。
- **比例**：3:4 或 1:1。

# 质量约束
- 不写空洞鸡汤，不伪造名人名言。
- 不用「最」「第一」「保证」等绝对化用语。
- 句子可以被单独截图传播，但不能脱离语境后造成误导。
- 输出纯 Markdown，不要解释创作过程。"#,
    },
    SeedPreset {
        id: "preset-video-storyboard",
        name: "视频分镜脚本",
        description: "3–5 分钟中视频分镜：场次、画面、口播、字幕、素材清单。",
        kind: "video",
        skill_ids: &["skill-story", "skill-structure", "skill-visual"],
        content: r#"# 创作目标
产出一份 3–5 分钟中视频分镜脚本，既能直接拍摄，也能作为剪辑指令。

# 结构
- 标题候选 3 个。
- 一句话内容承诺。
- 分镜表：镜号 | 时长 | 画面/景别 | 口播 | 字幕重点 | 所需素材。
- 开头 10 秒必须完成钩子；每 30 秒有一个信息推进或视觉变化。
- 结尾给行动建议，不诱导互动。

# 输出格式
Markdown 表格 + 尾部「素材清单」（按拍摄/版权/图形分类）。
标注哪些镜头可用图文卡替代，方便无实拍条件时降级制作。

# 质量约束
- 不虚构采访与数据。
- 如涉及 BGM/影视素材，提醒版权风险，优先推荐可商用素材。
- 总时长与分镜时长之和一致。"#,
    },
];

const WORKSPACE_README: &str = r#"# 媒材工坊工作区

这个目录里的所有内容都是普通文件，可以直接用 VS Code、Git 或任何编辑器管理。

- `ideas/`：灵感收件箱，每条想法一个 Markdown 文件。
- `projects/`：每个项目一个目录，内含 `article/`、`script/`、`illustration/`、`card/`、`video/` 五类素材。
- `.mediadesk/skills/`：技能库，定义创作方法。
- `.mediadesk/presets/`：按素材类型定义的创作预设。
- `.mediadesk/config.json`：本地配置（含 API 密钥，已默认被 `.gitignore` 忽略）。
- `.mediadesk/trash/`：删除的内容会先进入这里，可手动恢复。

## 给外部 AI 的提示

打开任一项目目录，阅读该目录下的 `AGENTS.md`。它说明了素材结构、frontmatter 字段与写作约束。
"#;

pub fn seed_skills() -> Vec<(SkillMeta, &'static str)> {
    SKILLS
        .iter()
        .map(|s| {
            (
                SkillMeta {
                    id: s.id.into(),
                    name: s.name.into(),
                    description: s.description.into(),
                    applies_to: s.applies_to.iter().map(|x| x.to_string()).collect(),
                    code: Vec::new(),
                },
                s.content,
            )
        })
        .collect()
}

pub fn seed_presets() -> Vec<(PresetMeta, &'static str)> {
    PRESETS
        .iter()
        .map(|p| {
            (
                PresetMeta {
                    id: p.id.into(),
                    name: p.name.into(),
                    description: p.description.into(),
                    kind: p.kind.into(),
                    skill_ids: p.skill_ids.iter().map(|x| x.to_string()).collect(),
                },
                p.content,
            )
        })
        .collect()
}

pub fn default_skill_ids_for_kind(kind: &str) -> Vec<String> {
    PRESETS
        .iter()
        .find(|p| p.kind == kind)
        .map(|p| p.skill_ids.iter().map(|x| x.to_string()).collect())
        .unwrap_or_default()
}

pub fn default_preset_id_for_kind(kind: &str) -> Option<String> {
    PRESETS
        .iter()
        .find(|p| p.kind == kind)
        .map(|p| p.id.to_string())
}

pub async fn seed_builtins(workspace: &Path) -> ApiResult<()> {
    let meta_dir = workspace.join(".mediadesk");
    for sub in ["skills", "presets", "trash"] {
        fs::create_dir_all(meta_dir.join(sub)).await?;
    }

    for (meta, body) in seed_skills() {
        let path = meta_dir.join("skills").join(format!("{}.md", meta.id));
        if !path.exists() {
            let raw = frontmatter::render(&meta, body)
                .map_err(|e| ApiError::internal(format!("写入内置技能失败：{e}")))?;
            atomic_write(&path, raw.as_bytes()).await?;
        }
    }
    for (meta, body) in seed_presets() {
        let path = meta_dir.join("presets").join(format!("{}.md", meta.id));
        if !path.exists() {
            let raw = frontmatter::render(&meta, body)
                .map_err(|e| ApiError::internal(format!("写入内置预设失败：{e}")))?;
            atomic_write(&path, raw.as_bytes()).await?;
        }
    }

    let marker = meta_dir.join("seed.json");
    let payload = json!({
        "version": SEED_VERSION,
        "seededAt": Utc::now().to_rfc3339(),
    });
    atomic_write(
        &marker,
        serde_json::to_string_pretty(&payload)
            .map_err(|e| ApiError::internal(e.to_string()))?
            .as_bytes(),
    )
    .await?;

    let readme = meta_dir.join("README.md");
    if !readme.exists() {
        atomic_write(&readme, WORKSPACE_README.as_bytes()).await?;
    }

    let gitignore = workspace.join(".gitignore");
    if !gitignore.exists() {
        atomic_write(
            &gitignore,
            b".mediadesk/config.json\n.mediadesk/trash/\n.DS_Store\n*.tmp\n",
        )
        .await?;
    }

    Ok(())
}
