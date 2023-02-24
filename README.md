# egui_cop_and_robbers

Visualizer for algorithms on the game of Cops and Robbers. Currently I've implemented Random and MENACE.

Cops and Robbers - [These](https://www.youtube.com/watch?v=9mJEu-j1KT0) [videos](https://www.youtube.com/watch?v=fXvN-pF76-E) are useful resources on the game of Cops and Robbers.

MENACE - [These](https://www.youtube.com/watch?v=R9c-_neaxeU) [videos](https://www.youtube.com/watch?v=KcmjOtkULi4) and [this](https://www.mscroggs.co.uk/blog/19) blog post are useful resources for the Menace algorithm.

Note: My MENACE implementation is a bit different -
- We start with 50 tokens for each move.
- We don't change any tokens in any box until the match is over. When it ends, we add 3 tokens if it's a win, else we remove 1 token.
- If the box gets empty, we reset it to 50 tokens for each move.

## App
<img width="248" alt="egui_cops_and_robbers_1" src="https://user-images.githubusercontent.com/91585022/221196572-7037fc17-50b8-4de5-833e-c2e13f639d49.PNG">
<img width="249" alt="egui_cops_and_robbers_2" src="https://user-images.githubusercontent.com/91585022/221196597-406e46a5-f011-445d-87e7-fd7e85d88d4b.PNG">
