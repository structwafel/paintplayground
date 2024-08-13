import { selectedColor } from "./color.js";

export class Grid {
    constructor() {
        this.cellSize = 10;
        this.gridWidth = 100;
        this.gridHeight = 100;
        this.scale = 1;
        this.translation = { x: 0, y: 0 };
        this.lastColoredSquare = null;

        this.mouseDown = false;
        this.lastMousePos = { x: 0, y: 0 };

        this.createGrid();
        this.addEventListeners();
    }

    createGrid() {
        const gridContainer = document.getElementById('gridContainer');
        gridContainer.style.border = '1px solid black';
        document.body.appendChild(gridContainer);

        for (let i = 0; i < this.gridWidth; i++) {
            for (let j = 0; j < this.gridHeight; j++) {
                const box = document.createElement('div');
                const index = i * this.gridWidth + j;
                box.id = index;
                box.className = 'gridBox';
                gridContainer.appendChild(box);
            }
        }

        this.gridContainer = gridContainer;
    }

    colorBox(index, color) {
        const box = this.gridContainer.querySelector(`[id='${index}']`);
        box.style.backgroundColor = color;
    }

    handleColoring(index) {
        if (index !== this.lastColoredSquare) {
            this.colorBox(index, selectedColor);
        }
    }

    handleZoom(event) {
        const zoomFactor = 1.1;
        const mouseX = event.clientX - this.gridContainer.getBoundingClientRect().left;
        const mouseY = event.clientY - this.gridContainer.getBoundingClientRect().top;

        if (event.deltaY < 0) {
            this.scale *= zoomFactor;
            this.translation.x -= mouseX * (zoomFactor - 1);
            this.translation.y -= mouseY * (zoomFactor - 1);
        } else {
            this.scale /= zoomFactor;
            this.translation.x += mouseX * (1 - 1 / zoomFactor);
            this.translation.y += mouseY * (1 - 1 / zoomFactor);
        }

        this.applyTransformations();
    }

    handlePanning(event) {
        if (event.button !== 1) {
            return;
        }
        const dx = event.clientX - this.lastMousePos.x;
        const dy = event.clientY - this.lastMousePos.y;

        this.translation.x += dx;
        this.translation.y += dy;

        this.lastMousePos = { x: event.clientX, y: event.clientY };

        this.applyTransformations();
    }

    applyTransformations() {
        this.gridContainer.style.transform = `scale(${this.scale}) translate(${this.translation.x}px, ${this.translation.y}px)`;
    }

    addEventListeners() {
        this.gridContainer.addEventListener('click', (event) => {
            if (event.target.classList.contains('gridBox')) {
                console.log(event.target.id);
                this.handleColoring(event.target.id);
            }
        });

        this.gridContainer.addEventListener('wheel', (event) => {
            event.preventDefault();
            this.handleZoom(event);
        });

        this.gridContainer.addEventListener('mousedown', (event) => {
            console.log('mousedown, button:', event.button);
            if (event.button === 0) {
                this.mouseDown = true;
                this.lastMousePos = { x: event.clientX, y: event.clientY };
            }
        });

        this.gridContainer.addEventListener('mouseup', (event) => {
            if (event.button === 0) {
                this.mouseDown = false;
            }
        });

        document.addEventListener('mouseup', (event) => {
            if (event.button === 0) {
                this.mouseDown = false;
            }
        });

        this.gridContainer.addEventListener('mousemove', (event) => {
            console.log('mousemove, button:', event.button, this.mouseDown);
            if (this.mouseDown && event.button === 0) {
                this.handleColoring(event.target.id);

            } else if (this.mouseDown && event.button === 1) {
                this.handlePanning(event);
            }
        });

        this.gridContainer.addEventListener('contextmenu', (event) => {
            event.preventDefault();
        });
    }
}