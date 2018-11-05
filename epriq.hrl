%% -*- mode:erlang -*-

-ifndef(HEADER_EPRIQ_PH).
-define(HEADER_EPRIQ_PH, true).

-record(epriq_queue_node, {priority :: epriq_queue_node_priority(),
                           value    :: epriq_queue_node_value()}).
-record(priring_heap_sub, {node :: epriq_queue_node(),
                           sub  :: priring_heap_sub()}).
-record(epriq_queue, {queue = null :: priring_heap_sub(),
                      len = 0      :: epriq_queue_len()}).

-type epriq_queue_len() :: integer().
-type epriq_queue_node_priority() :: integer().
-type epriq_queue_node_value() :: term().
-type epriq_queue_node() :: #epriq_queue_node{}.
-type priring_heap_sub() :: #priring_heap_sub{}.

-endif.
