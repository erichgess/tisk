# Action Chaining
Goal: to experiment with a design strategy.

Overview: Given the currrent design where the system is broken into two parts:

1. Apply command to in-memory state
2. Determine the effect on in-memory state and apply to persistent storage.

The question was: given this design, how to model more complex interactions between
changes to in-memory state and  committing to disk?  The current model decouples
and separates those two mechanics: e.g. you can change the commit action by changing
the CommandEffect value but you can't make a in-memory action dependent on the
result of the commit action.

With the constraint that the existing model of explicit separation between applying
commits to disk and applying changes to in-memory, how can more complex interactions
be modelled?

The solution: a play on the idea of "continuation passing".  The basic type
is now the `Chain` which allows a user to specify either an Effect that needs
to be committed based upon what their function did OR an `AndThen` chain
which includes the Effect of their function and a function to execute after
that Effect has been committed.

For example, the "Add task and checkout task" command would proceed as follows

1. AddAndCheckout would add a task and return the chain `(Write, CheckoutTask)`
2. The committer logic would see the Effect is `Write` and the `AndThen` is 
`CheckoutTask` so it would write to disk and then call the `CheckoutTask`
function.
3. `CheckoutTask` would do the work needed to checkout a task and return the
chain `Checkout(task id)`.  In this case there is no `AndThen` so this
is the terminal point of the chain.
4. The Committer logic would then commit the checkout to disk and the execution
would be done.
