import { selectedColor } from './color.js';
import { getWsUrl } from './utils.js';

let mouseDown = false;
let changedSquare = false;

export class CanvasManager {
    constructor(x, y) {
        this.socket = this.connect_ws(x, y);

        const canvas = document.getElementById('colorGrid');
        canvas.width = 1000;
        canvas.height = 1000;

        canvas.addEventListener('click', this.handle_coloring.bind(this));
        canvas.addEventListener('mousedown', (event) => {
            mouseDown = true;
            this.lastMousePos = { x: event.clientX, y: event.clientY };
        });
        canvas.addEventListener('mouseup', () => mouseDown = false);
        canvas.addEventListener('mousemove', (event) => {
            if (mouseDown) {
                this.handle_coloring(event);
                this.handle_panning(event);
            }
        });
        canvas.addEventListener('wheel', this.handle_zoom.bind(this));
        console.log("added event listener", canvas);
        this.canvas = canvas;

        const ctx = canvas.getContext('2d');
        this.ctx = ctx;

        this.cellSize = 10;
        this.gridWidth = 100;
        this.gridHeight = 100;
        this.colors = new Array(this.gridWidth * this.gridHeight).fill('grey');

        // Track the last colored square
        this.lastColoredSquare = { x: null, y: null };

        // Track transformations
        this.scale = 1;
        this.translation = { x: 0, y: 0 };
    }

    connect_ws(x, y) {
        const socket = new WebSocket(getWsUrl(x, y));
        socket.binaryType = 'arraybuffer';
        socket.onopen = function () {
            console.log('WebSocket connection established');
        };

        return socket;
    }

    draw_sqare(x, y, color) {
        this.ctx.fillStyle = color;
        this.ctx.fillRect(x * this.cellSize, y * this.cellSize, this.cellSize, this.cellSize);
    }

    draw_entire_grid() {
        this.ctx.save();
        this.ctx.setTransform(this.scale, 0, 0, this.scale, this.translation.x, this.translation.y);
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);

        for (let i = 0; i < this.gridWidth; i++) {
            for (let j = 0; j < this.gridHeight; j++) {
                this.draw_sqare(i, j, this.colors[i * this.gridWidth + j]);
            }
        }

        this.ctx.restore();
    }

    handle_coloring(event) {
        const rect = this.canvas.getBoundingClientRect();
        const x = Math.floor((event.clientX - rect.left - this.translation.x) / (this.cellSize * this.scale));
        const y = Math.floor((event.clientY - rect.top - this.translation.y) / (this.cellSize * this.scale));

        // Check if the current square is different from the last colored square
        if (x !== this.lastColoredSquare.x || y !== this.lastColoredSquare.y) {
            console.log("handling coloring");
            // Update the last colored square
            this.lastColoredSquare = { x, y };

            // Send the color to the server
            const color = selectedColor;
            this.colors[y * this.gridWidth + x] = color;
            this.draw_sqare(x, y, color);
            // this.socket.send(JSON.stringify({ x, y, color }));
        }
    }

    handle_zoom(event) {

        event.preventDefault();
        console.log("handling zoom");
        const zoomFactor = 1.1;
        const mouseX = event.clientX - this.canvas.getBoundingClientRect().left;
        const mouseY = event.clientY - this.canvas.getBoundingClientRect().top;

        if (event.deltaY < 0) {
            // Zoom in
            const newScale = this.scale * zoomFactor;
            console.log(newScale);
            if (newScale > 1) {
                this.scale = 1;
            } else {
                this.scale = newScale;
            }
            this.translation.x -= mouseX * (zoomFactor - 1);
            this.translation.y -= mouseY * (zoomFactor - 1);
        } else {
            // Zoom out

            const newScale = this.scale * zoomFactor;
            console.log(newScale);
            if (newScale < 1) {
                this.scale = 1;
            } else {
                this.scale = newScale;
            }
            this.translation.x += mouseX * (1 - 1 / zoomFactor);
            this.translation.y += mouseY * (1 - 1 / zoomFactor);
        }

        this.draw_entire_grid();
    }

    handle_panning(event) {
        // return if left mouse button is pressed
        if (event.buttons === 1) {
            return;
        }
        const dx = event.clientX - this.lastMousePos.x;
        const dy = event.clientY - this.lastMousePos.y;

        this.translation.x += dx;
        this.translation.y += dy;

        this.lastMousePos = { x: event.clientX, y: event.clientY };

        this.draw_entire_grid();
    }
}