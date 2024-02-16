# Generated by the gRPC Python protocol compiler plugin. DO NOT EDIT!
"""Client and server classes corresponding to protobuf-defined services."""
import grpc

import flexim_py.pb.connect_pb2 as connect__pb2


class FleximConnectStub(object):
    """Missing associated documentation comment in .proto file."""

    def __init__(self, channel):
        """Constructor.

        Args:
            channel: A grpc.Channel.
        """
        self.CreateBag = channel.unary_unary(
            "/flexim_connect.FleximConnect/CreateBag",
            request_serializer=connect__pb2.CreateBagRequest.SerializeToString,
            response_deserializer=connect__pb2.CreateBagResponse.FromString,
        )
        self.AppendData = channel.stream_unary(
            "/flexim_connect.FleximConnect/AppendData",
            request_serializer=connect__pb2.AppendDataRequest.SerializeToString,
            response_deserializer=connect__pb2.AppendDataResponse.FromString,
        )
        self.ListBags = channel.unary_unary(
            "/flexim_connect.FleximConnect/ListBags",
            request_serializer=connect__pb2.ListBagsRequest.SerializeToString,
            response_deserializer=connect__pb2.ListBagsResponse.FromString,
        )


class FleximConnectServicer(object):
    """Missing associated documentation comment in .proto file."""

    def CreateBag(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details("Method not implemented!")
        raise NotImplementedError("Method not implemented!")

    def AppendData(self, request_iterator, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details("Method not implemented!")
        raise NotImplementedError("Method not implemented!")

    def ListBags(self, request, context):
        """Missing associated documentation comment in .proto file."""
        context.set_code(grpc.StatusCode.UNIMPLEMENTED)
        context.set_details("Method not implemented!")
        raise NotImplementedError("Method not implemented!")


def add_FleximConnectServicer_to_server(servicer, server):
    rpc_method_handlers = {
        "CreateBag": grpc.unary_unary_rpc_method_handler(
            servicer.CreateBag,
            request_deserializer=connect__pb2.CreateBagRequest.FromString,
            response_serializer=connect__pb2.CreateBagResponse.SerializeToString,
        ),
        "AppendData": grpc.stream_unary_rpc_method_handler(
            servicer.AppendData,
            request_deserializer=connect__pb2.AppendDataRequest.FromString,
            response_serializer=connect__pb2.AppendDataResponse.SerializeToString,
        ),
        "ListBags": grpc.unary_unary_rpc_method_handler(
            servicer.ListBags,
            request_deserializer=connect__pb2.ListBagsRequest.FromString,
            response_serializer=connect__pb2.ListBagsResponse.SerializeToString,
        ),
    }
    generic_handler = grpc.method_handlers_generic_handler("flexim_connect.FleximConnect", rpc_method_handlers)
    server.add_generic_rpc_handlers((generic_handler,))


# This class is part of an EXPERIMENTAL API.
class FleximConnect(object):
    """Missing associated documentation comment in .proto file."""

    @staticmethod
    def CreateBag(
        request,
        target,
        options=(),
        channel_credentials=None,
        call_credentials=None,
        insecure=False,
        compression=None,
        wait_for_ready=None,
        timeout=None,
        metadata=None,
    ):
        return grpc.experimental.unary_unary(
            request,
            target,
            "/flexim_connect.FleximConnect/CreateBag",
            connect__pb2.CreateBagRequest.SerializeToString,
            connect__pb2.CreateBagResponse.FromString,
            options,
            channel_credentials,
            insecure,
            call_credentials,
            compression,
            wait_for_ready,
            timeout,
            metadata,
        )

    @staticmethod
    def AppendData(
        request_iterator,
        target,
        options=(),
        channel_credentials=None,
        call_credentials=None,
        insecure=False,
        compression=None,
        wait_for_ready=None,
        timeout=None,
        metadata=None,
    ):
        return grpc.experimental.stream_unary(
            request_iterator,
            target,
            "/flexim_connect.FleximConnect/AppendData",
            connect__pb2.AppendDataRequest.SerializeToString,
            connect__pb2.AppendDataResponse.FromString,
            options,
            channel_credentials,
            insecure,
            call_credentials,
            compression,
            wait_for_ready,
            timeout,
            metadata,
        )

    @staticmethod
    def ListBags(
        request,
        target,
        options=(),
        channel_credentials=None,
        call_credentials=None,
        insecure=False,
        compression=None,
        wait_for_ready=None,
        timeout=None,
        metadata=None,
    ):
        return grpc.experimental.unary_unary(
            request,
            target,
            "/flexim_connect.FleximConnect/ListBags",
            connect__pb2.ListBagsRequest.SerializeToString,
            connect__pb2.ListBagsResponse.FromString,
            options,
            channel_credentials,
            insecure,
            call_credentials,
            compression,
            wait_for_ready,
            timeout,
            metadata,
        )
