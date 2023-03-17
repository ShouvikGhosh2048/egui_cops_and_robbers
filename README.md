# egui_cop_and_robbers

Visualizer for algorithms on the game of Cops and Robbers. Currently I've implemented Random and MENACE.

Cops and Robbers - [These](https://www.youtube.com/watch?v=9mJEu-j1KT0) [videos](https://www.youtube.com/watch?v=fXvN-pF76-E) are useful resources on the game of Cops and Robbers.

MENACE - [These](https://www.youtube.com/watch?v=R9c-_neaxeU) [videos](https://www.youtube.com/watch?v=KcmjOtkULi4) and [this](https://www.mscroggs.co.uk/blog/19) blog post are useful resources for the Menace algorithm.

Note: My MENACE implementation is a bit different -
- We start with 50 tokens for each move.
- We don't change any tokens in any box until the match is over. When it ends, we add 3 tokens if it's a win, else we remove 1 token.
- If the box gets empty, we reset it to 50 tokens for each move.

## App
### Game selection
<img width="541" alt="CopsAndRobbers1" src="https://user-images.githubusercontent.com/91585022/225943519-e7b62c10-e2d4-4758-a587-a180bd15fe73.PNG">

### Graph editor
<img width="544" alt="CopsAndRobbers2" src="https://user-images.githubusercontent.com/91585022/225944695-fdc70e87-51b7-4ab9-ad64-4d92b17f16b3.PNG">

You can create your own graphs. Graphs require a name and at least one vertex.

Vertex mode: In the vertex mode you can create vertices by right clicking and move vertices by dragging them.

Edge mode: In the edge mode you can create edges by dragging the edge from one vertex to the other.

Delete: You can delete vertices/edges by selecting them and clicking the delete button.

You can create the graph / cancel the creation with the respective buttons.

### Game
<img width="541" alt="CopsAndRobbers3" src="https://user-images.githubusercontent.com/91585022/225947619-8dd7f888-c5ca-4909-afad-ef33218f4eae.PNG">

The game view has two parts - the game and the statistics.

The statistics panel consists of the current cop, current robber and the graph of the cop wins.

For MENACE algorithms, we have two types of bags - start bags (first move) and non start bags.

<img width="232" alt="CopsAndRobbers5" src="https://user-images.githubusercontent.com/91585022/225950632-d2828d04-412b-4555-a53c-e9f581c6f2d8.PNG">

For MENACE robber bags and non start MENACE cop bags, you can change the selected bag by choosing the object to edit, and then clicking the vertex you want.

You can sort the moves in descending order of the number of tokens.

<img width="262" alt="CopsAndRobbers6" src="https://user-images.githubusercontent.com/91585022/225951639-dc58cbf8-10ec-4da3-b662-20999e5ad3c9.PNG">

You can view the graph of the fraction of cops wins.
