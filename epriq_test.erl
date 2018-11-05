-module(epriq_test).

-author('MohsenMoqadam@yahoo.com').

-export([run/0,
         run/2]).

-include("epriq.hrl").

run() ->
    run(100, 10).

run(Size, PriorityRange) ->
    Seq = lists:seq(1, Size),
    Queue = lists:foldl(fun(Value, Queue0) ->
                                Priority = rand:uniform(PriorityRange),
                                Node = #epriq_queue_node{priority = Priority,
                                                         value = Value},
                                epriq:add_node(Node, Queue0)
                        end,
                        epriq:init(),
                        Seq),

    lists:foldl(fun(_, Queue0) ->
                        {ok, Node, Queue1} = epriq:get_node(Queue0),
                        io:format("===> Node: ~p~n", [Node]),
                        Queue1
                end,
                Queue,
                Seq),

    ok.
