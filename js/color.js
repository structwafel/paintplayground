export let selectedColor = 'red';
export const colorMapping = {
    'grey': 0,
    'red': 1,
    'green': 2,
    'blue': 3,
    'yellow': 4,
    'purple': 5,
    'orange': 6,
    'pink': 7,
    'brown': 8,
    'black': 9
};
export function colorFromNumber(number) {
    switch (number) {
        case 0:
            return 'grey';
        case 1:
            return 'red';
        case 2:
            return 'green';
        case 3:
            return 'blue';
        case 4:
            return 'yellow';
        case 5:
            return 'purple';
        case 6:
            return 'orange';
        case 7:
            return 'pink';
        case 8:
            return 'brown';
        case 9:
            return 'black';
        default:
            return 'grey';
    }
}
document.getElementById('color-picker').addEventListener('click', function (event) {
    selectedColor = event.target.id;
});