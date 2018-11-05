-module(epriq).

-author('MohsenMoqadam@yahoo.com').

-export([init/0,
         add_node/2,
         add_nodes/2,
         get_node/1,
         get_size/1,
         is_empty/1]).

-include("epriq.hrl").

%% ===
init() ->
    #epriq_queue{}.

%% ===
add_node(#epriq_queue_node{} = Node,
         #epriq_queue{queue = Queue, len = Len}) ->
    NewSub = #priring_heap_sub{node = Node, sub = []},
    #epriq_queue{queue = merge(NewSub, Queue),
                 len = Len + 1}.

%% ===
add_nodes([], #epriq_queue{} = Queue) ->
    Queue;
add_nodes([Node | Rest], #epriq_queue{} = Queue) ->
    add_nodes(Rest, add_node(Node, Queue)).

%% ===
merge(#epriq_queue{queue = Queue1, len = Len1},
      #epriq_queue{queue = Queue2, len = Len2}) ->
    #epriq_queue{queue = merge(Queue1, Queue2),
                 len = Len1 + Len2};
merge(null, #priring_heap_sub{} = Sub) -> Sub;
merge(#priring_heap_sub{} = Sub, null) -> Sub;
merge(#priring_heap_sub{node = Node1, sub = Node1Sub} = Sub1,
      #priring_heap_sub{node = Node2, sub = Node2Sub} = Sub2) ->
    Priority1 = Node1#epriq_queue_node.priority,
    Priority2 = Node2#epriq_queue_node.priority,
    if
        Priority1 > Priority2 ->
            #priring_heap_sub{node = Node1,
                              sub = [Sub2 | Node1Sub]};
        true ->
            #priring_heap_sub{node = Node2,
                              sub = [Sub1 | Node2Sub]}
    end.

%% ===
pair([]) -> null;
pair([Sub]) -> Sub;
pair([Sub1, Sub2 | Rest]) ->
    Sub3 = merge(Sub1, Sub2),
    merge(Sub3, pair(Rest)).

%% ===
get_node(#epriq_queue{queue = Queue, len = Len}) ->
    #priring_heap_sub{node = Node,
                      sub = Sub} = Queue,
    {ok, Node, #epriq_queue{queue = pair(Sub), len = Len - 1}}.

%% ===
get_size(#epriq_queue{len = Len}) -> Len.

%% ===
is_empty(#epriq_queue{len = 0}) -> true;
is_empty(#epriq_queue{}) -> false.
