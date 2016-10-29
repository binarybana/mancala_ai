# Mancala AI using Reinforcement Learning

This is a toy example using Reinforcement learning to (hopefully competently) play the game of Mancala (or more specifically the [Kalah](https://en.wikipedia.org/wiki/Kalah) derivative). 

Initial goals (phase one):
- Learn Rust enough to implement this project using it
- Implement a simple tabular temporal difference RL engine that can learn to play it.

Phase two through N:
- Try TD(\lambda) approaches with eligibilty traces
- Use function approximation for the value or state-action function (implement a little fully connected neural net interface or call out to Neon through python?)

## Project structure

(eventually):

- Driver code (main.rs)
- Simulator interface
  - Mancala world implementation
- RL code
