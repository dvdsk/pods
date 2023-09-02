use std::collections::HashSet;

use crate::podcasts::add::text;
use crate::{menu, Loading, Message, Page};

use tracing::{instrument, warn};

use iced::widget::{self, Column, Scrollable};
use traits::{DataUpdateVariant, Episode, EpisodeDetails, EpisodeId, PodcastId};

#[instrument(skip(state), ret)]
pub(crate) fn load(state: &mut crate::State, podcast_id: PodcastId) {
    let Some(podcast) = state
        .podcasts
        .iter()
        .find(|p| p.id == podcast_id) else {
            warn!("podcast with id: {podcast_id} got deleted after clicking view");
            return;
        };

    if state.podcast.as_ref().map(|p| p.id) != Some(podcast.id) {
        state.tx.view_episodes(podcast.id);
        state.podcast = Some(crate::Podcast {
            name: podcast.name.clone(),
            id: podcast.id,
            episodes: Vec::new(),
            details: None,
        });
    }

    let needed_data = HashSet::from([DataUpdateVariant::Episodes {
        podcast_id: podcast.id,
    }]);
    state.loading = Some(Loading {
        page: Page::Podcast {
            id: podcast.id,
            details: None,
        },
        needed_data,
    });
}

#[instrument(skip(state), ret)]
pub(crate) fn load_details(state: &mut crate::State, podcast_id: PodcastId, details: EpisodeId) {
    let needed_data = HashSet::from([DataUpdateVariant::EpisodeDetails {
        episode_id: details,
    }]);
    state.tx.view_episode_details(details);
    state.loading = Some(Loading {
        page: Page::Podcast {
            id: podcast_id,
            details: Some(details),
        },
        needed_data,
    });
}

pub(crate) fn view(
    mut column: widget::Column<'static, Message>,
    podcast: &crate::Podcast,
) -> widget::Column<'static, Message> {
    let mut list = Column::new();
    column = column.push(text(podcast.name.clone()));

    for episode in &podcast.episodes {
        if let Some(details) = &podcast.details {
            if details.episode_id == episode.id {
                list = view_details(list, podcast, episode, details);
                continue;
            }
        }

        let on_click = Message::ToPage(Page::Podcast {
            id: podcast.id,
            details: Some(episode.id),
        });
        list = list.push(menu::button(episode.name.clone(), on_click));
    }

    let list = Scrollable::new(list);
    column = column.push(list);

    column
}

pub(crate) fn view_details(
    mut list: widget::Column<'static, Message>,
    podcast: &crate::Podcast,
    episode: &Episode,
    details: &EpisodeDetails,
) -> widget::Column<'static, Message> {
    let on_click = Message::ToPage(Page::Podcast {
        id: podcast.id,
        details: None,
    });
    list = list.push(text(episode.name.clone()));
    list = list.push(menu::button(details.description.clone(), on_click));
    list
}
