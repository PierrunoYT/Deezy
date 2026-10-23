export interface AlbumResult {
  id: number;
  title: string;
  artist: string;
  artist_id: number;
  cover_small: string;
  cover_medium: string;
  nb_tracks: number;
}

export interface ArtistResult {
  id: number;
  name: string;
  picture_small: string;
  picture_medium: string;
  nb_album: number;
  nb_fan: number;
}

export interface PlaylistResult {
  id: number;
  title: string;
  creator: string;
  cover_small: string;
  cover_medium: string;
  nb_tracks: number;
}

export interface SelectedArtist {
  id: number;
  name: string;
  picture: string;
}

export interface SelectedPlaylist {
  id: number;
  title: string;
  cover: string;
  creator: string;
}

export type CatalogSearchType = 'tracks' | 'albums' | 'artists' | 'playlists';
/** Tabs that list the logged-in user's own library instead of search results. */
export type LibraryType = 'loved' | 'myPlaylists';
export type SearchType = CatalogSearchType | LibraryType;

export function isLibraryType(type: SearchType): type is LibraryType {
  return type === 'loved' || type === 'myPlaylists';
}

