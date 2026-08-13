export const COLOR_THEME_STORAGE_KEY = 'ocstats-color-theme';
export const DEFAULT_COLOR_THEME = 'pastel-pink';

export const colorThemes = [
	{
		id: 'pastel-pink',
		label: 'Pastel pink'
	},
	{
		id: 'monochrome',
		label: 'Monochrome'
	},
	{
		id: 'pastel-purple',
		label: 'Pastel purple'
	},
	{
		id: 'strong-purple',
		label: 'Stronger purple'
	},
	{
		id: 'green',
		label: 'Green'
	},
	{
		id: 'blue',
		label: 'Blue'
	}
] as const;

export type ColorTheme = (typeof colorThemes)[number]['id'];
