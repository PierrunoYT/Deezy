export type Provider = 'deezer' | 'spotify';
export type CatalogId = number | string;

export interface AlbumResult {
  id: CatalogId;
  title: string;
  artist: string;
  artist_id: CatalogId;
  cover_small: string;
  cover_medium: string;
  nb_tracks: number;
  release_date?: string;
  provider?: Provider;
}

export interface ArtistResult {
  id: CatalogId;
  name: string;
  picture_small: string;
  picture_medium: string;
  nb_album: number;
  nb_fan: number;
  provider?: Provider;
}

export interface PlaylistResult {
  id: CatalogId;
  title: string;
  creator: string;
  cover_small: string;
  cover_medium: string;
  nb_tracks: number;
  provider?: Provider;
}

export interface SelectedArtist {
  id: CatalogId;
  name: string;
  picture: string;
}

export interface SelectedPlaylist {
  id: CatalogId;
  title: string;
  cover: string;
  creator: string;
  provider?: Provider;
}

export type CatalogSearchType = 'tracks' | 'albums' | 'artists' | 'playlists';
/** Tabs that list the logged-in user's own library instead of search results. */
export type LibraryType = 'loved' | 'myPlaylists';
export type SearchType = CatalogSearchType | LibraryType;

export function isLibraryType(type: SearchType): type is LibraryType {
  return type === 'loved' || type === 'myPlaylists';
}
