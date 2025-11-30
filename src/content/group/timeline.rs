// use serde::Deserialize;

// #[derive(Debug, Deserialize)]
// #[serde(rename_all = "lowercase")]
// enum GroupBy {
//     Year,
//     Month,
//     Day,
// }

// impl Default for GroupBy {
//     fn default() -> Self {
//         GroupBy::Month
//     }
// }

// #[derive(Debug, Deserialize, Default)]
// pub struct TimelineOptions {
//     #[serde(default)] // 未提供时默认使用 Default::default()
//     pub group_by: GroupBy, // 默认就是 Month
// }
