use regex::Regex;
use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MediaResponse {
  pub Items: Vec<Item>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum Type {
  Movie,
  Series,
  Season,
  Episode,
  Special,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Item {
  Name: String,
  pub Id: String,
  pub IndexNumber: Option<u32>,
  pub ParentIndexNumber: Option<u32>,
  IndexNumberEnd: Option<u32>,
  pub Type: Type,
  SeriesName: Option<String>,
  pub SeriesId: Option<String>,
  SeasonName: Option<String>,
  pub SeasonId: Option<String>,
  pub MediaStreams: Option<Vec<MediaStream>>,
  pub CommunityRating: Option<f64>,
  pub RunTimeTicks: Option<u64>,
  PremiereDate: Option<String>,
  pub ProductionYear: Option<u32>,
  Status: Option<String>,
  EndDate: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MediaStream {
  pub Type: String,
  pub Language: Option<String>,
  pub Height: Option<u32>,
  pub IsInterlaced: bool,
}

pub trait LibraryTools {
  fn contains(&self, s: String) -> bool;
}

impl LibraryTools for Vec<Vec<Item>> {
  fn contains(&self, s: String) -> bool {
    for itemlist in self.iter() {
      for item in itemlist {
        if item.Id == s.clone() {
          return true;
        }
      }
    }
    false
  }
}

impl ToString for Type {
  fn to_string(&self) -> String {
    match self {
      Self::Movie => String::from("Movie"),
      Self::Episode => String::from("Episode"),
      Self::Season => String::from("Season"),
      Self::Series => String::from("Series"),
      Self::Special => String::from("Special"),
    }
  }
}

impl ToString for Item {
  fn to_string(&self) -> String {
    let time = if let (Some(start), Some(end)) = (self.PremiereDate.clone(), self.EndDate.clone()) {
      if start[0..4] == end[0..4] {
        format!("({})", &start[0..4])
      } else {
        format!("({}-{})", &start[0..4], &end[0..4])
      }
    } else if self.Status == Some(String::from("Continuing")) {
      format!(
        "({}-)",
        &self.PremiereDate.clone().unwrap_or(String::from("????"))[0..4]
      )
    } else if let Some(premiere_date) = &self.PremiereDate {
      format!("({})", &premiere_date[0..4])
    } else if let Some(production_year) = &self.ProductionYear {
      format!("({})", production_year)
    } else {
      "(???)".to_string()
    };
    let mut name: String;
    match self.Type {
      Type::Season | Type::Episode => name = self.SeriesName.clone().unwrap_or(String::from("???")),
      _ => name = self.Name.clone(),
    }
    if name.contains('(') {
      let re = Regex::new(r" \(\d{4}\)").unwrap();
      name = re.replace_all(&name, "").to_string();
    }

    match self.Type {
      Type::Movie | Type::Series => {
        format!("{} {}", name, time)
      },
      Type::Season => {
        format!("{} {} - {}", name, time, self.Name.clone())
      },
      Type::Episode => match self.IndexNumberEnd {
        Some(indexend) => {
          format!(
            "{} {} - S{:02}E{:02}-{:02} - {}",
            name,
            time,
            self.ParentIndexNumber.unwrap_or(0),
            self.IndexNumber.unwrap_or(0),
            indexend,
            self.Name
          )
        },
        None => {
          format!(
            "{} {} - S{:02}E{:02} - {}",
            name,
            time,
            self.ParentIndexNumber.unwrap_or(0),
            self.IndexNumber.unwrap_or(0),
            self.Name
          )
        },
      },
      _ => format!("{} {} (unknown media type)", self.Name, time),
    }
  }
}

pub struct Runtime(pub u64);

impl Runtime {
  pub fn fmt(&self) -> String {
    let time = (self.0 as f64) / 10000000.0;
    let formatted_runtime: String = if time > 60.0 {
      if (time / 60.0) > 60.0 {
        format!(
          "{:02}:{:02}:{:02}",
          ((time / 60.0) / 60.0).trunc(),
          ((((time / 60.0) / 60.0) - ((time / 60.0) / 60.).trunc()) * 60.0).trunc(),
          (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc()
        )
      } else {
        format!(
          "00:{:02}:{:02}",
          (time / 60.0).trunc(),
          (((time / 60.0) - (time / 60.0).trunc()) * 60.0).trunc()
        )
      }
    } else {
      format!("00:00:{time:02}")
    };
    formatted_runtime
  }
}

pub struct EpisodeInfo {
  pub Indexes: String,
  pub AudioLanguages: Vec<String>,
  pub SubtitleLanguages: Vec<String>,
  pub VideoResolutions: Vec<String>,
  pub Ratings: f64,
  pub TotalRuntime: Runtime,
}

pub fn get_episodes_info(episode_list: &mut [Item]) -> EpisodeInfo {
  let mut desc = String::new();
  let mut a_languages: Vec<String> = vec![];
  let mut s_languages: Vec<String> = vec![];
  let mut v_resolutions: Vec<String> = vec![];
  let mut ratings_sum: f64 = 0.0;
  let mut ratings_amount: u32 = 0;
  let mut total_runtime: u64 = 0;
  let mut current_start: i32 = -1;
  for (i, episode) in episode_list.iter().enumerate() {
    if episode.MediaStreams.is_some() {
      for x in episode.MediaStreams.clone().unwrap() {
        if x.Type == "Video" {
          let resolution: String;

          let scan_type: char = if x.IsInterlaced { 'i' } else { 'p' };

          if let Some(height) = x.Height {
            resolution = height.to_string() + &scan_type.to_string();
          } else {
            resolution = String::from("?") + &scan_type.to_string();
          }

          if !v_resolutions.contains(&resolution) {
            v_resolutions.push(resolution);
          }
        } else if x.Type == "Audio" {
          let lang = x.Language.unwrap_or("?".to_string());
          if !a_languages.contains(&lang) {
            a_languages.push(lang);
          }
        } else if x.Type == "Subtitle" {
          let lang = x.Language.unwrap_or("?".to_string());
          if !s_languages.contains(&lang) {
            s_languages.push(lang);
          }
        }
      }
    }

    if let Some(rating) = episode.CommunityRating {
      ratings_sum += rating;
      ratings_amount += 1;
    }

    if let Some(runtime) = episode.RunTimeTicks {
      total_runtime += runtime;
    }

    let index_start = episode.IndexNumber.unwrap() as i32;
    let index_end = if let Some(end) = episode.IndexNumberEnd {
      end as i32
    } else {
      index_start
    };
    let item_name_full = match episode.IndexNumberEnd {
      Some(indexend) => {
        format!(
          "S{:02}E{:02}-{:02}",
          episode.ParentIndexNumber.unwrap_or(0),
          episode.IndexNumber.unwrap_or(0),
          indexend
        )
      },
      None => {
        format!(
          "S{:02}E{:02}",
          episode.ParentIndexNumber.unwrap_or(0),
          episode.IndexNumber.unwrap_or(0)
        )
      },
    };
    let item_name_end = match episode.IndexNumberEnd {
      Some(indexend) => {
        format!(
          "S{:02}E{:02}",
          episode.ParentIndexNumber.unwrap_or(0),
          indexend
        )
      },
      None => {
        format!(
          "S{:02}E{:02}",
          episode.ParentIndexNumber.unwrap_or(0),
          episode.IndexNumber.unwrap_or(0)
        )
      },
    };
    let item_name_start = format!(
      "S{:02}E{:02}",
      episode.ParentIndexNumber.unwrap_or(0),
      episode.IndexNumber.unwrap_or(0)
    );

    if episode_list.len() - 1 == i {
      if current_start == -1 {
        desc.push_str(&item_name_full.to_string());
      } else {
        desc.push_str(&format!("-{}", item_name_end));
      }
    } else if i == 0 || current_start == -1 {
      if episode_list[i + 1].IndexNumber.unwrap() as i32 != index_end + 1 {
        desc.push_str(&format!("{}, ", item_name_full));
        current_start = -1;
        continue;
      } else {
        desc.push_str(&item_name_start);
      }
    } else if episode_list[i + 1].IndexNumber.unwrap() as i32 != index_end + 1 {
      desc.push_str(&format!("-{}, ", item_name_end));
      current_start = -1;
      continue;
    }
    current_start = index_start;
  }

  EpisodeInfo {
    Indexes: desc,
    AudioLanguages: a_languages,
    SubtitleLanguages: s_languages,
    VideoResolutions: v_resolutions,
    Ratings: ratings_sum / ratings_amount as f64,
    TotalRuntime: Runtime(total_runtime),
  }
}
