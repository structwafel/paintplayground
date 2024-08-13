export const host = window.location.host;
// if it is secure use wss
export function getWsUrl(x, y) {
    if (window.location.protocol === 'https:') {
        return `wss://${host}/ws/${x}/${y}`;
    } else {
        return `ws://${host}/ws/${x}/${y}`;
    }
}

