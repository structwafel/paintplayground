import { colorFromNumber } from './color.js';

export class Ws {
    constructor(x, y, applyColoringUpdate) {
        this.x = x;
        this.y = y;

        this.applyColor = applyColoringUpdate;
        const socket = this.connect_ws(x, y);
        socket.onmessage = (event) => {
            this.handleMessage(event.data);
        }

        this.socket = socket;
    }

    connect_ws(x, y) {
        const socket = new WebSocket(getWsUrl(x, y));
        socket.binaryType = 'arraybuffer';
        socket.onopen = function () {
            console.log('WebSocket connection established');
        };

        return socket;
    }

    connect_and_extrac_ws(x, y) {
        const socket = this.connect_ws(x, y);
        socket.onmessage = (event) => {
            this.handleMessage(event.data);
        }


        return socket;
    }

    // todo do this server side, so it checks if you are allowed to move to that square
    move(x, y) {
        this.socket.close();

        this.x += x;
        this.y += y;

        this.socket = this.connect_and_extrac_ws(this.x, this.y);
    }

    handleMessage(data) {
        console.log('Received message with length', data.byteLength / 8);
        // all will be binary.
        const view = new DataView(data);
        const messageType = view.getUint8(0);
        switch (messageType) {
            // receiving the entire chunk

            case 1:
                console.log('Received chunk');
                for (let i = 1; i < data.byteLength; i++) {
                    const byte = view.getUint8(i);
                    const color1 = byte >> 4;
                    const color2 = byte & 0x0F;

                    const doublePackedColor1 = colorFromNumber(color1);
                    const doublePackedColor2 = colorFromNumber(color2);

                    this.applyColor((i - 1) * 2, doublePackedColor1);
                    this.applyColor((i - 1) * 2 + 1, doublePackedColor2);
                }
                break;
            // chunks updates, a packed u64 with 60bit index and 4bit color
            case 2:
                for (let i = 1; i < data.byteLength; i += 8) {
                    const packed = view.getBigUint64(i, true);
                    const index = Number(packed >> 4n);
                    const color = Number(packed & 0b1111n);
                    this.applyColor(index, color);
                }
                break
            // chunk not found
            // requested a chunk that does not exist, disconnectm
            case 3:
                console.error('Chunk not found');
                this.socket.close();
                break;
            // too many chunks loaded, wait for a bit
            case 4:
                alert('Too many chunks loaded, wait a bit');
                this.socket.close();
                break;
            default:
                console.error('Unknown message type');
        }

    }
}

export const host = window.location.host;
// if it is secure use wss
export function getWsUrl(x, y) {
    if (window.location.protocol === 'https:') {
        return `wss://${host}/ws/${x}/${y}`;
    } else {
        return `ws://${host}/ws/${x}/${y}`;
    }
}
